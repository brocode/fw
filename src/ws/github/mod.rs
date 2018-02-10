use errors::AppError;
use github_gql::client::Github;
use github_gql::query::Query;
use serde_json::Value;

pub fn github_api(token: String) -> Result<GithubApi, AppError> {
  let client = Github::new(token)?;

  Ok(GithubApi { client })
}

pub struct GithubApi {
  client: Github,
}

struct PageResult {
  repository_names: Vec<String>,
  next_cursor: Option<String>,
}

impl GithubApi {
  pub fn list_repositories(&mut self, org: &str) -> Result<Vec<String>, AppError> {
    let initial_page = self.page_repositories(org, None)?;
    let mut initial_names = initial_page.repository_names;

    let mut next: Option<String> = initial_page.next_cursor;
    while next.clone().is_some() {
      let next_repos = self.page_repositories(org, next.clone())?;
      initial_names.extend(next_repos.repository_names);
      next = next_repos.next_cursor;
    }

    Ok(initial_names)
  }
  fn page_repositories(&mut self, org: &str, after: Option<String>) -> Result<PageResult, AppError> {
    let after_refinement = after
      .map(|a| format!(", after:\\\"{}\\\"", a))
      .unwrap_or_else(|| "".to_owned());
    let (_, status, json) = self
      .client
      .query::<Value>(&Query::new_raw("query {organization(login: \\\"".to_owned() + org + "\\\"){repositories(first: 100" + &after_refinement +
                                     ") {nodes {name} pageInfo {endCursor hasNextPage}}}}"))?;
    if !status.is_success() {
      Err(AppError::RuntimeError(format!("GitHub repository query failed for {}, got status {} with json {:?}",
                                         org,
                                         status,
                                         json)))
    } else {
      let data_json = json
        .ok_or(AppError::InternalError("organization repository list has no json"))?;
      let nodes_json_value: Value = data_json
        .pointer("/data/organization/repositories/nodes")
        .ok_or(AppError::InternalError("no nodes in repository json"))?
        .to_owned();
      let nodes_json: Vec<Value> = nodes_json_value
        .as_array()
        .ok_or(AppError::InternalError("nodes in repository json is not an array"))?
        .to_owned();
      let maybe_names = nodes_json
        .into_iter()
        .flat_map(|n| n.pointer("/name").map(|name| name.to_owned()));
      let names: Vec<String> = maybe_names
        .flat_map(|name| name.as_str().map(|reference| reference.to_owned()))
        .collect();

      let has_next: bool = data_json
        .pointer("/data/organization/repositories/pageInfo/hasNextPage")
        .ok_or(AppError::InternalError("no page info (hasNextPage) in repository json"))?
        .as_bool()
        .ok_or(AppError::InternalError("page info (hasNextPage) in repository json is not a boolean"))?;

      let end_cursor: String = data_json
        .pointer("/data/organization/repositories/pageInfo/endCursor")
        .ok_or(AppError::InternalError("no page info (endCursor) in repository json"))?
        .as_str()
        .ok_or(AppError::InternalError("page info (endCursor) in repository json is not a string"))?
        .to_owned();

      Ok(PageResult {
           repository_names: names,
           next_cursor: if has_next { Some(end_cursor) } else { None },
         })
    }
  }
}

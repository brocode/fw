use errors::*;
use github_gql::client::Github;
use github_gql::query::Query;
use serde_json::Value;

pub fn github_api(token: String) -> Result<GithubApi> {
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
  pub fn list_repositories(&mut self, org: &str) -> Result<Vec<String>> {
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
  fn page_repositories(&mut self, org: &str, after: Option<String>) -> Result<PageResult> {
    let after_refinement = after.map(|a| format!(", after:\\\"{}\\\"", a)).unwrap_or_else(|| "".to_owned());
    let (_, status, json) = self.client.query::<Value>(&Query::new_raw(
      "query {organization(login: \\\"".to_owned()
        + org
        + "\\\"){repositories(first: 100"
        + &after_refinement
        + ") {nodes {name} pageInfo {endCursor hasNextPage}}}}",
    ))?;
    if !status.is_success() {
      Err(
        ErrorKind::RuntimeError(format!(
          "GitHub repository query failed for {}, got status {} with json {:?}",
          org, status, json
        )).into(),
      )
    } else {
      let data_json = json.chain_err(|| ErrorKind::InternalError("organization repository list has no json".to_string()))?;
      let nodes_json_value: Value = data_json
        .pointer("/data/organization/repositories/nodes")
        .chain_err(|| ErrorKind::InternalError("no nodes in repository json".to_string()))?
        .to_owned();
      let nodes_json: Vec<Value> = nodes_json_value
        .as_array()
        .chain_err(|| ErrorKind::InternalError("nodes in repository json is not an array".to_string()))?
        .to_owned();
      let maybe_names = nodes_json.into_iter().flat_map(|n| n.pointer("/name").map(|name| name.to_owned()));
      let names: Vec<String> = maybe_names.flat_map(|name| name.as_str().map(|reference| reference.to_owned())).collect();

      let has_next: bool = data_json
        .pointer("/data/organization/repositories/pageInfo/hasNextPage")
        .chain_err(|| ErrorKind::InternalError("no page info (hasNextPage) in repository json".to_string()))?
        .as_bool()
        .chain_err(|| ErrorKind::InternalError("page info (hasNextPage) in repository json is not a boolean".to_string()))?;

      let end_cursor: String = data_json
        .pointer("/data/organization/repositories/pageInfo/endCursor")
        .chain_err(|| ErrorKind::InternalError("no page info (endCursor) in repository json".to_string()))?
        .as_str()
        .chain_err(|| ErrorKind::InternalError("page info (endCursor) in repository json is not a string".to_string()))?
        .to_owned();

      Ok(PageResult {
        repository_names: names,
        next_cursor: if has_next { Some(end_cursor) } else { None },
      })
    }
  }
}

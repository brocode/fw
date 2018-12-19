use crate::errors::*;
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

#[derive(Serialize, Deserialize, Debug)]
struct OrganizationQueryResponse {
  data: OrganizationQueryResponseData,
}

#[derive(Serialize, Deserialize, Debug)]
struct OrganizationQueryResponseData {
  organization: OrganizationRepositoriesResponseData,
}

#[derive(Serialize, Deserialize, Debug)]
struct OrganizationRepositoriesResponseData {
  repositories: RepositoriesResponseData,
}

#[derive(Serialize, Deserialize, Debug)]
struct RepositoriesResponseData {
  nodes: Vec<Repository>,
  #[serde(rename = "pageInfo")]
  page_info: PageInfo,
}

#[derive(Serialize, Deserialize, Debug)]
struct Repository {
  name: String,
  #[serde(rename = "isArchived")]
  is_archived: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct PageInfo {
  #[serde(rename = "endCursor")]
  end_cursor: String,
  #[serde(rename = "hasNextPage")]
  has_next_page: bool,
}

impl GithubApi {
  pub fn list_repositories(&mut self, org: &str, include_archived: bool) -> Result<Vec<String>> {
    let initial_page = self.page_repositories(org, None, include_archived)?;
    let mut initial_names = initial_page.repository_names;

    let mut next: Option<String> = initial_page.next_cursor;
    while next.clone().is_some() {
      let next_repos = self.page_repositories(org, next.clone(), include_archived)?;
      initial_names.extend(next_repos.repository_names);
      next = next_repos.next_cursor;
    }

    Ok(initial_names)
  }
  fn page_repositories(&mut self, org: &str, after: Option<String>, include_archived: bool) -> Result<PageResult> {
    let after_refinement = after.map(|a| format!(", after:\\\"{}\\\"", a)).unwrap_or_else(|| "".to_owned());
    let (_, status, json) = self.client.query::<Value>(&Query::new_raw(
      "query {organization(login: \\\"".to_owned()
        + org
        + "\\\"){repositories(first: 100"
        + &after_refinement
        + ") {nodes {name, isArchived} pageInfo {endCursor hasNextPage}}}}",
    ))?;
    if !status.is_success() {
      Err(
        ErrorKind::RuntimeError(format!(
          "GitHub repository query failed for {}, got status {} with json {:?}",
          org, status, json
        ))
        .into(),
      )
    } else {
      let data_json = json.chain_err(|| ErrorKind::InternalError("organization repository list has no json".to_string()))?;
      let response: OrganizationQueryResponse =
        serde_json::from_value(data_json).chain_err(|| ErrorKind::InternalError("Failed to parse github response".to_string()))?;

      let repositories: Vec<Repository> = response.data.organization.repositories.nodes;
      let repo_names: Vec<String> = repositories
        .into_iter()
        .filter(|r| include_archived || !r.is_archived)
        .map(|r| r.name)
        .collect();
      Ok(PageResult {
        repository_names: repo_names,
        next_cursor: if response.data.organization.repositories.page_info.has_next_page {
          Some(response.data.organization.repositories.page_info.end_cursor)
        } else {
          None
        },
      })
    }
  }
}

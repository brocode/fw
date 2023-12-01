// some of the code is from here: https://github.com/mgattozzi/github-rs/tree/master/github-gql-rs
// this package seems unmaintained at the moment. Also it is basically just a small http client wrapper.

use crate::errors::AppError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub fn github_api(token: &str) -> Result<GithubApi, AppError> {
    let client = reqwest::blocking::Client::new();
    Ok(GithubApi {
        client,
        token: token.to_string(),
    })
}

pub struct GithubApi {
    client: reqwest::blocking::Client,
    token: String,
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
    fn query<T: DeserializeOwned>(&self, query: &str) -> Result<T, AppError> {
        //escaping new lines and quotation marks for json
        let mut escaped = query.to_string();
        escaped = escaped.replace('\n', "\\n");
        escaped = escaped.replace('\"', "\\\"");

        let mut q = String::from("{ \"query\": \"");
        q.push_str(&escaped);
        q.push_str("\" }");

        let res = self
            .client
            .post("https://api.github.com/graphql")
            .body(reqwest::blocking::Body::from(q))
            .header("Content-Type", "application/json")
            .header("User-Agent", "github-rs")
            .header("Authorization", format!("token {}", self.token))
            .send()?;

        if res.status().is_success() {
            res.json::<T>().map_err(|e| AppError::RuntimeError(format!("Failed to parse response: {}", e)))
        } else {
            Err(AppError::RuntimeError(format!("Bad status from github {}", res.status())))
        }
    }

    pub fn list_repositories(&mut self, org: &str, include_archived: bool) -> Result<Vec<String>, AppError> {
        let initial_page = self.page_repositories(org, None, include_archived)?;
        let mut initial_names = initial_page.repository_names;

        let mut next: Option<String> = initial_page.next_cursor;
        while next.is_some() {
            let next_repos = self.page_repositories(org, next.clone(), include_archived)?;
            initial_names.extend(next_repos.repository_names);
            next = next_repos.next_cursor;
        }

        Ok(initial_names)
    }
    fn page_repositories(&mut self, org: &str, after: Option<String>, include_archived: bool) -> Result<PageResult, AppError> {
        let after_refinement = after.map(|a| format!(", after:\"{}\"", a)).unwrap_or_else(|| "".to_owned());
        let response: OrganizationQueryResponse = self.query(
            &("query {organization(login: \"".to_owned()
                + org
                + "\"){repositories(first: 100"
                + &after_refinement
                + ") {nodes {name, isArchived} pageInfo {endCursor hasNextPage}}}}"),
        )?;
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

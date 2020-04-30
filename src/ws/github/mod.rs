// some of the code is from here: https://github.com/mgattozzi/github-rs/tree/master/github-gql-rs
// this package seems unmaintained at the moment. Also it is basically just a small http client wrapper.

use crate::errors::AppError;
use serde_json::Value;

// Tokio/Future Imports
use futures::future::ok;
use futures::{Future, Stream};
use tokio_core::reactor::Core;

use hyper::client::Client;
use hyper::StatusCode;
use hyper::{self, HeaderMap};
type HttpsConnector = hyper_rustls::HttpsConnector<hyper::client::HttpConnector>;

use serde::de::DeserializeOwned;

use std::cell::RefCell;
use std::rc::Rc;

use hyper::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use hyper::Request;

/// Used to query information from the GitHub API to possibly be used in
/// a `Mutation` or for information to make decisions with how to interact.
pub struct Query {
  pub(crate) query: String,
}

impl Query {
  /// Create a new `Query` using the given value as the input for the query to
  /// GitHub. Any other methods used will assume the `String` is empty. This
  /// is a shortcut for doing:
  ///
  /// ```no_test
  /// let q = Query::new();
  /// q.raw_query("my query which won't work");
  /// ```
  ///
  /// as
  ///
  /// ```no_test
  /// let q = Query::new_raw("my query which won't work");
  /// ```
  pub fn new_raw<T>(q: &T) -> Self
  where
    T: ToString,
  {
    Self { query: q.to_string() }
  }
}

impl IntoGithubRequest for Query {
  fn into_github_req(&self, token: &str) -> Result<Request<hyper::Body>, AppError> {
    //escaping new lines and quotation marks for json
    let mut escaped = (&self.query).to_string();
    escaped = escaped.replace("\n", "\\n");
    escaped = escaped.replace("\"", "\\\"");

    let mut q = String::from("{ \"query\": \"");
    q.push_str(&escaped);
    q.push_str("\" }");
    let mut req = Request::builder()
      .method("POST")
      .uri("https://api.github.com/graphql")
      .body(q.into())
      .map_err(|err| AppError::RuntimeError(format!("Unable to for URL to make the request. Cause: {:?}", err)))?;

    let token = format!("token {}", token);
    {
      let headers = req.headers_mut();
      headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
      headers.insert(USER_AGENT, HeaderValue::from_static("github-rs"));
      headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&token).map_err(|err| AppError::RuntimeError(format!("token parse. Cause: {:?}", err)))?,
      );
    }
    Ok(req)
  }
}

pub trait IntoGithubRequest {
  fn into_github_req(&self, token: &str) -> Result<hyper::Request<hyper::Body>, AppError>;
}

/// Struct used to make calls to the Github API.
pub struct Github {
  token: String,
  core: Rc<RefCell<Core>>,
  client: Rc<Client<HttpsConnector, hyper::Body>>,
}

impl Clone for Github {
  fn clone(&self) -> Self {
    Self {
      token: self.token.clone(),
      core: self.core.clone(),
      client: self.client.clone(),
    }
  }
}

impl Github {
  /// Create a new Github client struct. It takes a type that can convert into
  /// a `String` (`&str` or `Vec<u8>` for example). As long as the function is
  /// given a valid API Token your requests will work.
  pub fn create<T>(token: &T) -> Result<Self, AppError>
  where
    T: ToString,
  {
    let core = Core::new()?;
    let client = Client::builder().build(HttpsConnector::new(4));
    Ok(Self {
      token: token.to_string(),
      core: Rc::new(RefCell::new(core)),
      client: Rc::new(client),
    })
  }

  pub fn query<T>(&mut self, query: &Query) -> Result<(HeaderMap, StatusCode, Option<T>), AppError>
  where
    T: DeserializeOwned,
  {
    self.run(query)
  }

  fn run<T, I>(&mut self, request: &I) -> Result<(HeaderMap, StatusCode, Option<T>), AppError>
  where
    T: DeserializeOwned,
    I: IntoGithubRequest,
  {
    let mut core_ref = self
      .core
      .try_borrow_mut()
      .map_err(|error| AppError::RuntimeError(format!("Unable to get mutable borrow to the event loop. Cause: {:?}", error)))?;
    let client = &self.client;
    let work = client.request(request.into_github_req(&self.token)?).and_then(|res| {
      let header = res.headers().clone();
      let status = res.status();
      res
        .into_body()
        .fold(Vec::new(), |mut v, chunk| {
          v.extend(&chunk[..]);
          ok::<_, hyper::Error>(v)
        })
        .map(move |chunks| {
          if chunks.is_empty() {
            Ok((header, status, None))
          } else {
            Ok((
              header,
              status,
              Some(serde_json::from_slice(&chunks).map_err(|error| AppError::RuntimeError(format!("Failed to parse response body. Cause: {:?}", error)))?),
            ))
          }
        })
    });
    core_ref
      .run(work)
      .map_err(|error| AppError::RuntimeError(format!("Failed to execute request. Cause: {:?}", error)))?
  }
}

pub fn github_api(token: &str) -> Result<GithubApi, AppError> {
  let client = Github::create(&token)?;

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
  pub fn list_repositories(&mut self, org: &str, include_archived: bool) -> Result<Vec<String>, AppError> {
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
  fn page_repositories(&mut self, org: &str, after: Option<String>, include_archived: bool) -> Result<PageResult, AppError> {
    let after_refinement = after.map(|a| format!(", after:\"{}\"", a)).unwrap_or_else(|| "".to_owned());
    let (_, status, json) = self.client.query::<Value>(&Query::new_raw(
      &("query {organization(login: \"".to_owned()
        + org
        + "\"){repositories(first: 100"
        + &after_refinement
        + ") {nodes {name, isArchived} pageInfo {endCursor hasNextPage}}}}"),
    ))?;
    if !status.is_success() {
      Err(AppError::RuntimeError(format!(
        "GitHub repository query failed for {}, got status {} with json {:?}",
        org, status, json
      )))
    } else {
      let data_json = json.ok_or(AppError::InternalError("organization repository list has no json"))?;

      let response: OrganizationQueryResponse = serde_json::from_value(data_json)?;
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

use crate::{
    database::Database,
    types::{ErrorMessage, IndexQueryParams, ServerError},
    util::{
        minijinja_format_as_hms, minijinja_format_as_ymd, minijinja_format_title, tomorrow_midnight,
    },
};
use anyhow::{Context, Error, Result};
use log::error;
use minijinja::{context, Environment};
use rust_embed::RustEmbed;
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::runtime::Runtime;
use warp::{
    http::HeaderValue,
    hyper::StatusCode,
    path::Tail,
    reject,
    reply::{self, Response},
    Filter, Rejection, Reply,
};

const DEFAULT_SEARCH_INTERVAL: i64 = 3_600_000 * 24 * 30; // 30 days
#[derive(RustEmbed)]
#[folder = "static"]
struct Asset;

async fn serve_file(path: Tail) -> Result<impl Reply, Rejection> {
    let path = path.as_str();
    let asset = Asset::get(path).ok_or_else(reject::not_found)?;
    let mut res = Response::new(asset.data.into());

    let mime = mime_guess::from_path(path).first_or_octet_stream();
    if let Ok(v) = HeaderValue::from_str(mime.as_ref()) {
        res.headers_mut().insert("content-type", v);
    }

    Ok(res)
}

struct Server {
    db: Arc<Database>,
    addr: SocketAddr,
}

impl Server {
    fn try_new(addr: String, db_filepath: String) -> Result<Self> {
        Ok(Self {
            db: Arc::new(Database::open(db_filepath).context("open db")?),
            addr: addr.parse()?,
        })
    }

    fn with_db(
        db: Arc<Database>,
    ) -> impl Filter<Extract = (Arc<Database>,), Error = Infallible> + Clone {
        warp::any().map(move || db.clone())
    }

    async fn details(db: Arc<Database>, day: i64) -> Result<impl Reply, Rejection> {
        let start = day;
        let end = start + 3_600_000 * 24;
        let visit_details = db
            .select_visits(start, end, None)
            .map_err(ServerError::from)?;

        let asset = Asset::get("details.html").unwrap();
        let index_tmpl: &str =
            std::str::from_utf8(&asset.data).map_err(|e| ServerError::from(Error::from(e)))?;
        let mut env = Environment::new();
        env.add_template("details", index_tmpl)
            .map_err(|e| ServerError::from(Error::from(e)))?;

        env.add_function("format_as_ymd", minijinja_format_as_ymd);
        env.add_function("format_as_hms", minijinja_format_as_hms);
        env.add_function("format_title", minijinja_format_title);
        let tmpl = env.get_template("details").unwrap();
        let body = tmpl
            .render(context!(
                day => day,
                visit_details => visit_details,
                version => clap::crate_version!(),
            ))
            .map_err(|e| ServerError::from(Error::from(e)))?;

        Ok(reply::html(body))
    }

    async fn index(
        db: Arc<Database>,
        query_params: IndexQueryParams,
    ) -> Result<impl Reply, Rejection> {
        let end = query_params.end.unwrap_or_else(|| tomorrow_midnight() - 1);
        let start = query_params
            .start
            .unwrap_or_else(|| tomorrow_midnight() - DEFAULT_SEARCH_INTERVAL);
        let keyword = query_params.keyword;

        let daily_counts = db
            .select_daily_count(start, end, keyword.clone())
            .context("daily_count")
            .map_err(ServerError::from)?;
        let (min_time, max_time) = db
            .select_min_max_time()
            .context("min_max_time")
            .map_err(ServerError::from)?;
        let title_top100 = db
            .select_title_top100(start, end, keyword.clone())
            .context("title_top100")
            .map_err(ServerError::from)?;
        let domain_top100 = db
            .select_domain_top100(start, end, keyword.clone())
            .context("domain_top100")
            .map_err(ServerError::from)?;

        let asset = Asset::get("index.html").unwrap();
        let index_tmpl: &str =
            std::str::from_utf8(&asset.data).map_err(|e| ServerError::from(Error::from(e)))?;
        let mut env = Environment::new();
        env.add_template("index", index_tmpl)
            .map_err(|e| ServerError::from(Error::from(e)))?;

        let tmpl = env.get_template("index").unwrap();
        let body = tmpl
            .render(context!(
                min_time => min_time,
                max_time => max_time,
                start => start,
                end => end,
                daily_counts => daily_counts,
                title_top100 => title_top100,
                domain_top100 => domain_top100,
                keyword => keyword.unwrap_or_else(|| "".to_string()),
                version => clap::crate_version!(),
            ))
            .map_err(|e| ServerError::from(Error::from(e)))?;

        Ok(reply::html(body))
    }

    // https://github.com/ItsNothingPersonal/warp-postgres-example/blob/main/src/main.rs#L63
    fn serve(&self) -> Result<()> {
        let index = warp::path::end()
            .and(Self::with_db(self.db.clone()))
            .and(warp::query::<IndexQueryParams>())
            .and_then(Self::index);

        let detail = Self::with_db(self.db.clone())
            .and(warp::path!("details" / i64))
            .and_then(Self::details);

        let static_route = warp::path("static")
            .and(warp::path::tail())
            .and_then(serve_file);

        let routes = detail
            .or(index)
            .or(static_route)
            .recover(Self::handle_rejection);

        let rt = Runtime::new().context("tokio runtime build")?;
        rt.block_on(async {
            warp::serve(routes).run(self.addr).await;
        });
        Ok(())
    }

    async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
        let code;
        let message;

        if err.is_not_found() {
            code = StatusCode::NOT_FOUND;
            message = "NOT_FOUND";
        } else if let Some(ServerError { e }) = err.find() {
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = e;
        } else {
            error!("unhandled rejection: {:?}", err);
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = "UNHANDLED_REJECTION";
        }

        let json = warp::reply::json(&ErrorMessage {
            message: message.into(),
            code: code.as_u16(),
        });

        Ok(warp::reply::with_status(json, code))
    }
}

pub fn serve(addr: String, db_filepath: String) -> Result<()> {
    let server = Server::try_new(addr, db_filepath)?;
    server.serve()
}

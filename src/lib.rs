// Copyright (C) 2022  Alex Crawford
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashMap;
use std::iter::{self, FromIterator};
use worker::{Date, Env, Headers, Request, Response, Result, Router};

#[worker::event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    #[cfg(feature = "panic")]
    console_error_panic_hook::set_once();

    Router::new()
        .get_async("/world/:id", |req, ctx| async move {
            let id = match ctx.param("id") {
                Some(id) => id,
                None => {
                    worker::console_debug!(
                        "{} - [{}]: missing map identifier",
                        Date::now().to_string(),
                        req.path()
                    );
                    return Response::error("Bad Request", 400);
                }
            };

            let hitmaps = match ctx.kv("hitmaps") {
                Ok(kv) => kv,
                Err(err) => {
                    worker::console_error!(
                        "{} - [{}]: failed to lookup kv ({})",
                        Date::now().to_string(),
                        req.path(),
                        err
                    );
                    return Response::error("Internal Server Error", 500);
                }
            };

            let mut hitmap = match hitmaps.get(id).json().await {
                Ok(Some(hitmap)) => hitmap,
                Ok(None) => HashMap::new(),
                Err(err) => {
                    worker::console_error!(
                        "{} - [{}]: failed to get hitmap ({})",
                        Date::now().to_string(),
                        req.path(),
                        err
                    );
                    return Response::error("Internal Server Error", 500);
                }
            };

            match req.cf().country() {
                Some(country) => {
                    hitmap
                        .entry(country)
                        .and_modify(|count| *count += 1)
                        .or_insert(1);
                }

                None => worker::console_warn!(
                    "{} - [{}]: unknown country of request",
                    Date::now().to_string(),
                    req.path()
                ),
            }

            match hitmaps.put(id, &hitmap) {
                Ok(put) => {
                    if let Err(err) = put.execute().await {
                        worker::console_error!(
                            "{} - [{}]: failed to execute put ({})",
                            Date::now().to_string(),
                            req.path(),
                            err
                        );
                    }
                }
                Err(err) => worker::console_error!(
                    "{} - [{}]: failed to put ({})",
                    Date::now().to_string(),
                    req.path(),
                    err
                ),
            }

            let total: isize = hitmap.values().sum();
            let style: String = hitmap
                .iter()
                .map(|(key, val)| {
                    format!(
                        ".{} {{ fill: #b9{:02x}b9; }}\n",
                        key.to_lowercase(),
                        0xb9 - val * 0xb9 / total
                    )
                })
                .chain(iter::once(String::from("</style>")))
                .collect();

            Response::ok(include_str!("../assets/world.svg").replace("</style>", &style)).map(
                |res| {
                    res.with_headers(Headers::from_iter(
                        [("content-type", "image/svg+xml")].iter(),
                    ))
                },
            )
        })
        .run(req, env)
        .await
}

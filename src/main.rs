use std::{error::Error, net::SocketAddr};

use hyper_req_exts::{
    prelude::Response,
    routerify::{prelude::RequestExt, Router},
    start_server,
};
use reqwest::header::CONTENT_TYPE;

#[tokio::main]
async fn main() {
    let input_html = r#"<form action="/" id="sign_form"><input type="text" autofocus id="callsign"><input type="submit" id="sub" value="Ara"></form>
<script>
window.sign_form.onsubmit = function(e) {
    e.preventDefault();
    var sign = window.callsign.value;
    if (sign) {
        window.location.href = "/" + sign;
    }
};
</script>

                "#;
    let addr: SocketAddr = "127.0.0.1:64380".parse().unwrap();
    eprintln!("Listening on {}", addr);
    start_server(
        addr,
        Router::<String, String>::builder()
            .get("/:callsign", |r| {
                let input_html = input_html.to_string();
                async move {
                    let callsign = r.param("callsign").unwrap();
                    let (name, dmr) = tokio::join!(get_name(callsign), get_dmr_data(callsign));
                    let (name, dmr) = (
                        name.unwrap_or_default(),
                        dmr.unwrap_or_default().unwrap_or_default(),
                    );
                    Ok(Response::builder()
                        .header(CONTENT_TYPE, "text/html; charset=UTF-8")
                        .body::<String>(format!(
                            r#"
                            <style>
                            table, th, td {{
                                border: 1px solid black;
                                border-collapse: collapse;
                            }}
                            table{{
                            width: 100%;
                            text-align: center;
                            margin:auto auto;
                            }}
                            </style>
                            <table>
                                <thead>
                                    <tr>
                                        <th>çağrı işareti</td>
                                        <th>isim(callbook)</td>
                                        <th>isim(dmr)</td>
                                        <th>şehir</td>
                                        <th>ülke</td>
                                        <th>dmrid</td>
                                        <th>bölge</td>
                                    </tr>
                                </thead>
                            <tbody>
                                <tr>
                                    <td>{callsign}</td>
                                    <td>{name}</td>
                                    <td>{dmr_fname} {dmr_surname}</td>
                                    <td>{dmr_city}</td>
                                    <td>{dmr_country}</td>
                                    <td>{dmr_id}</td>
                                    <td>{dmr_state}</td>
                                </tr>
                            </tbody>
                            </table>
                            {input_html}
                            "#,
                            callsign = callsign,
                            name = name,
                            dmr_fname = dmr.fname,
                            dmr_surname = dmr.surname,
                            dmr_city = dmr.city,
                            dmr_country = dmr.country,
                            dmr_id = dmr.id,
                            dmr_state = dmr.state
                        ))
                        .unwrap())
                }
            })
            .get("/", |_| {
                let input_html = input_html.to_string();
                async move {
                    Ok(Response::builder()
                        .status(200)
                        .header(CONTENT_TYPE, "text/html; charset=UTF-8")
                        .body(input_html)
                        .unwrap())
                }
            })
            .any(|_| {
                let input_html = input_html.to_string();
                async move { Ok(Response::builder().status(200).body(input_html).unwrap()) }
            })
            .options("/*", |_| async move {
                Ok(Response::builder()
                    .status(404)
                    .body("".to_string())
                    .unwrap())
            })
            .err_handler(|err| async move {
                eprintln!("Error: {}", err);
                Response::builder()
                    .status(500)
                    .body("Internal Server Error".to_string())
                    .unwrap()
            })
            .build()
            .unwrap(),
    )
    .await;
}
#[derive(Clone, Debug, serde::Deserialize, Default)]
struct DmrData {
    #[serde(default)]
    pub fname: String,
    #[serde(default)]
    pub surname: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub id: usize,
    #[serde(default)]
    pub state: String,
}
async fn get_dmr_data(
    callsign: &str,
) -> Result<Option<DmrData>, Box<dyn Error + 'static + Send + Sync>> {
    #[derive(Debug, serde::Deserialize)]
    struct Response {
        results: Vec<DmrData>,
    }
    let url = format!("https://radioid.net/api/dmr/user/?callsign={callsign}");
    let body = reqwest::Client::new()
        .get(&url)
        .send()
        .await?
        .json::<Response>()
        .await?;
    Ok(body.results.get(0).cloned())
}
async fn get_name(callsign: &str) -> Result<String, Box<dyn Error + 'static + Send + Sync>> {
    //curl 'http://www.tacallbook.org/cgi-bin/bul1.cgi?ara=TA3KRT' \
    //-H 'Referer: http://www.tacallbook.org/call.shtml' > out.html
    let url = format!(
        "http://www.tacallbook.org/cgi-bin/bul1.cgi?ara={}",
        callsign
    );
    let body = reqwest::Client::new()
        .get(&url)
        .header("Referer", "http://www.tacallbook.org/call.shtml")
        .send()
        .await?
        .text()
        .await?;
    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r#">(.*?[\w ]+)</s"#).unwrap();
        // static ref RE: regex::Regex = regex::Regex::new(r#"strong>([\w ]+)</strong>"#).unwrap();
    };
    let name = RE
        .captures(&body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or_default();
    Ok(name.to_string())
}

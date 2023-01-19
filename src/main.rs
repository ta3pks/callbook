use std::{error::Error, net::SocketAddr};

use hyper_req_exts::{
    prelude::Response,
    routerify::{prelude::RequestExt, Router},
    start_server,
};
use reqwest::header::CONTENT_TYPE;

#[tokio::main]
async fn main() {
    let input_html = r#"<form action="/" id="sign_form"><input type="text" id="callsign"><input type="submit" id="sub" value="Ara"></form>
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
                    let name = get_name(callsign).await.unwrap();
                    Ok(Response::builder()
                        .header(CONTENT_TYPE, "text/html; charset=UTF-8")
                        .body(if name.is_empty() {
                            format!("{} için kayıt bulunamadı.{input_html}", callsign)
                        } else {
                            format!("{} - {}{input_html}", callsign, name)
                        })
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
async fn get_name(callsign: &str) -> Result<String, Box<dyn Error>> {
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

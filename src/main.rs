use clokwerk::{AsyncScheduler, TimeUnits};
use dotenv::dotenv;
use log::{debug, error, info, warn, LevelFilter};
use rand::seq::SliceRandom;
use regex::Regex;
use reqwest::header;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use std::env;
use std::io::{self, IsTerminal, Write};
use std::time::Duration;

async fn checkin_s1() -> Result<(), Box<dyn std::error::Error>> {
    let url_list = [
        "https://bbs.saraba1st.com/2b/forum-151-1.html",
        "https://bbs.saraba1st.com/2b/home.php?mod=spacecp&ac=credit&showcredit=1",
    ];
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "Cookie",
        header::HeaderValue::from_str(&env::var("cookie").expect("未找到cookie"))?,
    );
    let client = reqwest::ClientBuilder::new()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        )
        .default_headers(headers)
        .build()?;
    let random_url = url_list.choose(&mut rand::thread_rng()).unwrap();
    debug!("访问URL:{}", random_url);
    let resp = client.get(*random_url).send().await?.text().await?;
    if resp.contains("抱歉，您尚未登录，没有权限访问该版块")
        || resp.contains("您需要先登录才能继续本操作")
    {
        error!("未登录");
        return Err("未登录".into());
    }
    fn get_number(text: &str) -> Option<i32> {
        let re = Regex::new(r"积分: (\d+)").unwrap();
        if let Some(captures) = re.captures(&text) {
            let number_str = captures.get(1).unwrap().as_str();
            let number: i32 = number_str.parse().unwrap();
            Some(number)
        } else {
            None
        }
    }
    info!("刷新成功 当前积分:{}", get_number(&resp).unwrap());
    fn checkin_url(text: &str) -> Option<String> {
        let re =
            Regex::new(r"study_daily_attendance-daily_attendance\.html\?formhash=\w*").unwrap();
        if let Some(captures) = re.captures(&text) {
            Some("https://bbs.saraba1st.com/2b/".to_owned() + captures.get(0).unwrap().as_str())
        } else {
            None
        }
    }
    if let Some(url) = checkin_url(&resp) {
        let resp = client.get(url).send().await?.text().await?;
        if resp.contains("签到成功") {
            info!("签到成功");
        } else if resp.contains("已签到,请不要重新签到！") {
            warn!("今日已签到");
        } else {
            error!("签到未知错误");
        }
    }
    // println!("{:#?}", resp);
    Ok(())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    checkin_s1().await.expect("S1签到测试失败");
    let mut scheduler = AsyncScheduler::new();
    scheduler
        .every(
            env::var("interval")
                .unwrap_or("60".to_string())
                .parse::<u32>()?
                .seconds(),
        )
        .run(|| async {
            let _ = checkin_s1().await;
        });
    tokio::spawn(async move {
        loop {
            scheduler.run_pending().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
    println!("CheckinBot v{}", env!("CARGO_PKG_VERSION"));
    if io::stdin().is_terminal() {
        println!("Press 'q' to quit");
        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            let mut buf = String::new();
            io::stdin()
                .read_line(&mut buf)
                .expect("Failed to read line");
            match buf.trim() {
                "q" => break,
                _ => continue,
            }
        }
    } else {
        println!("Running in non-interactive mode");
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    Ok(())
}

fn main() {
    dotenv().ok();
    let config = ConfigBuilder::new()
        .set_time_offset_to_local()
        .unwrap()
        .build();
    let log_level = match env::var("log_level")
        .unwrap_or("debug".to_string())
        .as_str()
    {
        "off" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Debug,
    };
    TermLogger::init(log_level, config, TerminalMode::Mixed, ColorChoice::Auto).unwrap();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _ = async_main().await;
        })
}

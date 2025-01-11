use std::time::Duration;
use tokio::time::sleep;

pub async fn retry_requests(
    request_to_make: reqwest::Request,
    client: &reqwest::Client,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut retries = 0;
    let mut delay = Duration::from_millis(500); // Initial delay of 1 second
    let max_retries = 3;

    let mut result = client.execute(request_to_make.try_clone().unwrap()).await;

    result = loop {
        let status = result.as_ref().unwrap().status();

        if result.is_ok() && status.is_success() {
            println!("Request to {} successful", request_to_make.url());
            break result;
        }
        if retries >= max_retries {
            println!(
                "Request to {} failed after {} retries",
                request_to_make.url(),
                retries
            );
            break result;
        }
        println!(
            "Retrying request to {}, attempt: {}",
            request_to_make.url(),
            retries
        );
        sleep(delay).await;
        retries += 1;
        delay = Duration::from_millis(delay.as_millis() as u64 + 500);
        result = client.execute(request_to_make.try_clone().unwrap()).await;
    };
    result
}

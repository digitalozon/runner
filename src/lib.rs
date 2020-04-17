extern crate jsonpath_lib as jsonpath;

use std::fs::File;
use std::io::{BufReader};
use serde::{Deserialize, Serialize};
use actix_web::{
    http,client::Client
};
use ansi_term::Style;
use ansi_term::Colour::{Red, Green, Fixed};
use std::str::FromStr;


const DEFAULT_HTTP_METHOD : &'static str = "GET";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestItem {
    pub description : String,
    pub path: Option<String>,
    pub method: Option<String>,
    pub json_data: Option<JsonTestData>,
    pub status: i16,
    pub matches: Option<Vec<Match>>
}

impl TestItem {
    pub fn full_path(&self, abs_path: &String) -> String {
        match &self.path {
            Some(rel) => { format!("{}{}", abs_path, rel) }
            _  => { abs_path.clone() }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestCollection {
    pub base_url: String,
    pub tests: Vec<TestItem>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonTestData {
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub path: String,
    pub value: String,
    pub is_length: Option<bool>,
}

impl Match {

    /// Return tuple ( success, selected_values )
    pub fn validate(&self, json_resp_str: &String) -> (bool, String) {

        let json = serde_json::from_str(json_resp_str).unwrap();
        let selected_values = jsonpath::select(&json, &self.path.as_str()).unwrap();

        let is_length = self.is_length.unwrap_or(false);
        if is_length {
            return (self.value.parse::<usize>().unwrap() == selected_values.len(), format!("{:?}", selected_values.len()));
        } else {
            return (vec![self.value.as_str()] == selected_values, format!("{:?}", selected_values));
        }
    }
}

pub struct TestRunner {}

impl TestRunner {
    pub fn new() -> Self {
        // Load json data...

        TestRunner {}
    }

    pub async fn is_server_running(&self) -> std::io::Result<bool> {
        Ok(true)
    }


    /// Runs test from one json file
    pub async fn run_json_file(&self, path: &'static str) -> bool {


        println!("Loading json file: {}", path);
        let mut passed = true;

        let file = File::open(path).expect("Loading json file");
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `User`.
        let test_collection : TestCollection = serde_json::from_reader(reader).expect("Parsing json file");
        let base_url = test_collection.base_url;

        let client = Client::default();

        for test_item in test_collection.tests {
            let item_passed = self.run_item( &client, &test_item, &base_url ).await;
            passed = passed && item_passed;
        }


        passed
    }

    /// Runs single item, parsed from json  file
    async fn run_item( &self, client: &Client, item: &TestItem, base_url : &String ) -> bool {

        let test_url = item.full_path(base_url);
        let default_method = &DEFAULT_HTTP_METHOD.to_string();
        let method = item.method.as_ref().unwrap_or(default_method);

        println!("{}", "");
        println!("{}  {}",
                 Style::new().bold().underline().paint(&item.description),
                 Style::new().underline().paint(method));


        println!("URL: {}", &test_url);

        let mut res = match &item.json_data {
            Some(sending_json) => {
                println!("Sending JSON: {}", Fixed(242).paint(&sending_json.raw));
                client
                    .request(get_http_method_from_str(&method), &test_url)
                    .header("Content-Type", "application/json")
                    .send_body(&sending_json.raw)
                    .await.expect("Getting response")
            }
            _ => {
                client
                    .request(get_http_method_from_str(&method), &test_url)
                    .header("Content-Type", "application/json")
                    .send()
                    .await.expect("Getting response")
            }
        };


        // Calculate overall success
        let success_status = res.status().as_u16() == (item.status as u16);

        if !success_status {
            println!("Response status ( expected != actual ) : {} != {}",
                     Fixed(242).paint(format!("{}", item.status)),
                     Red.bold().paint(res.status().as_str())
             );
        } else {
            println!("Response status: {}",
                   Green.normal().paint(res.status().as_str()));
        }


        // read response body
        let body = res.body().await.unwrap();
        let testbody = String::from_utf8(body.to_vec()).unwrap();
        let body_str = format!("{:?}", body);


        println!("Response JSON: {}", Fixed(242).paint(&body_str));

        // Matches validation
        let matches_pass = match &item.matches {
            Some(match_list) => {
                let mut i_match_pass = true;
                println!("Matches: ");
                for match_item in match_list {
                    let validation = match_item.validate(&testbody);
                    i_match_pass = i_match_pass && validation.0;
                    if !i_match_pass {
                        println!(" - {} != {} {}",
                                 match_item.path,
                                 Red.normal().paint(&match_item.value),
                                 Fixed(242).paint(&validation.1)
                        );
                        return false
                    }else{
                        println!(" - {} == {} {}",
                                 match_item.path,
                                 Green.normal().paint(&match_item.value),
                                 Fixed(242).paint(&validation.1)
                        );
                    }
                }
                i_match_pass
            }
            None => { true }
        };

        let overall_item_pass =  success_status && matches_pass;
        self.print_item_result(overall_item_pass);

        overall_item_pass

    }

    /// Prints success or error at the end of the item test response.
    fn print_item_result(&self, success: bool) {
        if success {
            println!("{}",
                     Green.bold().paint("Success!"));
        } else {
            println!("{}",
                     Red.bold().paint("Error!"));
        }
    }
}

fn get_http_method_from_str( method: &String ) -> http::Method {
    http::Method::from_str(method.as_str()).expect("Getting method")
}


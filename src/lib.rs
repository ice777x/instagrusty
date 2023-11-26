use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn tagged_user(users: &Vec<Value>) -> Vec<User> {
    users
        .iter()
        .map(|user| User {
            id: user["node"]["user"]["id"].as_str().unwrap().to_owned(),
            full_name: user["node"]["user"]["full_name"]
                .as_str()
                .unwrap()
                .to_owned(),
            username: user["node"]["user"]["username"]
                .as_str()
                .unwrap()
                .to_owned(),
            image: user["node"]["user"]["profile_pic_url"]
                .as_str()
                .unwrap()
                .to_owned(),
            is_verified: user["node"]["user"]["is_verified"].as_bool().unwrap(),
        })
        .collect::<Vec<User>>()
}

#[derive(Serialize, Deserialize)]
pub struct Instagram {
    pub url: String,
}

#[async_trait]
pub trait Downloader {
    async fn download(&self) -> Result<Option<Post>, Box<dyn std::error::Error>>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Media {
    pub id: String,
    pub shortcode: String,
    pub thumbnail: String,
    pub resources: Vec<Source>,
    pub video: Option<String>,
    pub video_duration: Option<f32>,
    pub is_video: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub src: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub shortcode: String,
    pub typename: String,
    pub user: User,
    pub tagged_user: Vec<User>,
    pub caption: Vec<String>,
    pub media: Vec<Media>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub image: String,
    pub full_name: String,
    pub username: String,
    pub is_verified: bool,
}

pub trait Utils {
    fn new(url: &str) -> Self;
    fn regex(url: &str) -> Option<String>;
}

impl Utils for Instagram {
    fn new(url: &str) -> Self {
        Instagram {
            url: Instagram::regex(url).unwrap_or(String::new()),
        }
    }
    fn regex(url: &str) -> Option<String> {
        let re = Regex::new(
            r"((?:https?:\/\/)?(?:www\.)?instagram\.com\/(?:p|reel)\/(?<id>[^/?#&]+)).*",
        )
        .unwrap();
        match re.captures(url) {
            Some(c) => Some(c.name("id").unwrap().as_str().to_string()),
            None => None,
        }
    }
}

#[async_trait]

impl Downloader for Instagram {
    async fn download(&self) -> Result<Option<Post>, Box<dyn std::error::Error>> {
        if self.url == "" {
            return Ok(None);
        }

        let res = reqwest::get(format!("https://www.instagram.com/graphql/query/?query_hash=cf28bf5eb45d62d4dc8e77cdb99d750d&variables={{%22shortcode%22:%22{}%22}}",self.url)).await?.text().await?;
        let json: Value = serde_json::from_str(&res).unwrap();
        if json["data"]["shortcode_media"].is_null() {
            return Ok(None);
        }
        let data = &json["data"]["shortcode_media"];
        let user = User {
            id: data["owner"]["id"].as_str().unwrap().to_owned(),
            image: data["owner"]["profile_pic_url"]
                .as_str()
                .unwrap()
                .to_owned(),
            full_name: data["owner"]["full_name"].as_str().unwrap().to_owned(),
            username: data["owner"]["username"].as_str().unwrap().to_owned(),
            is_verified: data["owner"]["is_verified"].as_bool().unwrap(),
        };
        let tagged_user = tagged_user(
            data["edge_media_to_tagged_user"]["edges"]
                .as_array()
                .unwrap(),
        );
        let caption = data["edge_media_to_caption"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|cap| cap["node"]["text"].as_str().unwrap().to_string())
            .collect::<Vec<String>>();
        let media: Vec<Media>;
        if data["__typename"].as_str().unwrap() == "GraphSidecar" {
            media = data["edge_sidecar_to_children"]["edges"]
                .as_array()
                .unwrap()
                .iter()
                .map(|edge| Media {
                    id: edge["node"]["id"].as_str().unwrap().to_string(),
                    shortcode: edge["node"]["shortcode"].as_str().unwrap().to_string(),
                    thumbnail: edge["node"]["display_url"].as_str().unwrap().to_string(),
                    resources: edge["node"]["display_resources"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|rsrc| Source {
                            src: rsrc["src"].as_str().unwrap().to_string(),
                            width: rsrc["config_width"].as_u64().unwrap() as u32,
                            height: rsrc["config_height"].as_u64().unwrap() as u32,
                        })
                        .collect::<Vec<Source>>(),
                    video: if edge["node"]["is_video"].as_bool().unwrap() {
                        Some(edge["node"]["video_url"].as_str().unwrap().to_string())
                    } else {
                        None
                    },
                    video_duration: None,
                    is_video: edge["node"]["is_video"].as_bool().unwrap(),
                })
                .collect::<Vec<Media>>();
        } else {
            media = vec![Media {
                id: data["id"].as_str().unwrap().to_string(),
                shortcode: data["shortcode"].as_str().unwrap().to_string(),
                thumbnail: data["display_url"].as_str().unwrap().to_string(),
                resources: data["display_resources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|rsrc| Source {
                        src: rsrc["src"].as_str().unwrap().to_string(),
                        width: rsrc["config_width"].as_u64().unwrap() as u32,
                        height: rsrc["config_height"].as_u64().unwrap() as u32,
                    })
                    .collect::<Vec<Source>>(),
                video: if data["is_video"].as_bool().unwrap() {
                    Some(data["video_url"].as_str().unwrap().to_string())
                } else {
                    None
                },
                video_duration: if !data["video_duration"].is_null() {
                    Some(data["video_duration"].as_f64().unwrap() as f32)
                } else {
                    None
                },
                is_video: data["is_video"].as_bool().unwrap(),
            }];
        }
        Ok(Some(Post {
            id: data["id"].as_str().unwrap().to_owned(),
            shortcode: data["shortcode"].as_str().unwrap().to_owned(),
            typename: data["__typename"].as_str().unwrap().to_owned(),
            user,
            tagged_user,
            caption,
            media,
        }))
    }
}

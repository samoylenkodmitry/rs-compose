use compose_core::LaunchedEffect;
use compose_ui::{
    composable, Brush, Button, Color, Column, ColumnSpec, CornerRadii, LinearArrangement, Modifier,
    Size, Spacer, Text,
};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct Post {
    #[serde(rename = "userId")]
    user_id: i32,
    id: i32,
    title: String,
    body: String,
}

#[derive(Clone, Debug, PartialEq)]
enum FetchState {
    Idle,
    Loading,
    Success(Vec<Post>),
    Error(String),
}

#[composable]
pub fn web_fetch_tab() {
    let fetch_state = compose_core::useState(|| FetchState::Idle);
    let fetch_request = compose_core::useState(|| 0u64);
    let fetch_key = fetch_request.get();

    {
        let state_handle = fetch_state.clone();
        LaunchedEffect!(fetch_key, move |scope| {
            if fetch_key == 0 {
                return;
            }
            let state_for_bg = state_handle.clone();
            state_handle.set(FetchState::Loading);

            scope.launch_background(
                move |token| {
                    use std::time::Duration;

                    if token.is_cancelled() {
                        return Err("Cancelled".to_string());
                    }

                    // Fetch data from JSONPlaceholder API
                    match reqwest::blocking::Client::builder()
                        .timeout(Duration::from_secs(10))
                        .build()
                    {
                        Ok(client) => {
                            match client
                                .get("https://jsonplaceholder.typicode.com/posts")
                                .send()
                            {
                                Ok(response) => {
                                    if token.is_cancelled() {
                                        return Err("Cancelled".to_string());
                                    }

                                    match response.json::<Vec<Post>>() {
                                        Ok(posts) => {
                                            // Only return first 5 posts for brevity
                                            Ok(posts.into_iter().take(5).collect())
                                        }
                                        Err(e) => Err(format!("Failed to parse JSON: {}", e)),
                                    }
                                }
                                Err(e) => Err(format!("Request failed: {}", e)),
                            }
                        }
                        Err(e) => Err(format!("Failed to create HTTP client: {}", e)),
                    }
                },
                move |result: Result<Vec<Post>, String>| match result {
                    Ok(posts) => state_for_bg.set(FetchState::Success(posts)),
                    Err(err) => state_for_bg.set(FetchState::Error(err)),
                },
            );
        });
    }

    Column(
        Modifier::empty()
            .padding(32.0)
            .background(Color(0.08, 0.12, 0.20, 1.0))
            .rounded_corners(24.0)
            .padding(20.0),
        ColumnSpec::default(),
        move || {
            Text(
                "Web Data Fetcher",
                Modifier::empty()
                    .padding(12.0)
                    .background(Color(1.0, 1.0, 1.0, 0.08))
                    .rounded_corners(16.0),
            );

            Spacer(Size {
                width: 0.0,
                height: 16.0,
            });

            Text(
                "Fetches posts from JSONPlaceholder API",
                Modifier::empty()
                    .padding(8.0)
                    .background(Color(0.5, 0.7, 0.9, 0.2))
                    .rounded_corners(12.0),
            );

            Spacer(Size {
                width: 0.0,
                height: 16.0,
            });

            // Fetch button
            let fetch_request_for_button = fetch_request.clone();
            Button(
                Modifier::empty()
                    .rounded_corners(16.0)
                    .draw_behind(|scope| {
                        scope.draw_round_rect(
                            Brush::solid(Color(0.2, 0.45, 0.9, 1.0)),
                            CornerRadii::uniform(16.0),
                        );
                    })
                    .padding(12.0),
                move || {
                    fetch_request_for_button.update(|val| *val += 1);
                },
                || {
                    Text("Fetch Data", Modifier::empty().padding(6.0));
                },
            );

            Spacer(Size {
                width: 0.0,
                height: 20.0,
            });

            // Display state
            let current_state = fetch_state.get();
            match current_state {
                FetchState::Idle => {
                    Text(
                        "Click 'Fetch Data' to load posts from the web",
                        Modifier::empty()
                            .padding(16.0)
                            .background(Color(0.2, 0.2, 0.3, 0.5))
                            .rounded_corners(14.0),
                    );
                }
                FetchState::Loading => {
                    Column(
                        Modifier::empty()
                            .padding(16.0)
                            .background(Color(0.2, 0.3, 0.5, 0.6))
                            .rounded_corners(14.0),
                        ColumnSpec::default(),
                        || {
                            Text("Loading...", Modifier::empty().padding(8.0));
                            Spacer(Size {
                                width: 0.0,
                                height: 8.0,
                            });
                            Text(
                                "Fetching data from JSONPlaceholder API",
                                Modifier::empty().padding(4.0),
                            );
                        },
                    );
                }
                FetchState::Success(ref posts) => {
                    Column(
                        Modifier::empty()
                            .padding(16.0)
                            .background(Color(0.1, 0.3, 0.2, 0.6))
                            .rounded_corners(14.0),
                        ColumnSpec::new().vertical_arrangement(LinearArrangement::SpacedBy(12.0)),
                        {
                            let posts = posts.clone();
                            move || {
                                Text(
                                    format!("Successfully fetched {} posts:", posts.len()),
                                    Modifier::empty()
                                        .padding(8.0)
                                        .background(Color(0.2, 0.7, 0.4, 0.5))
                                        .rounded_corners(10.0),
                                );

                                for post in &posts {
                                    Column(
                                        Modifier::empty()
                                            .padding(12.0)
                                            .background(Color(0.15, 0.18, 0.25, 0.9))
                                            .rounded_corners(12.0),
                                        ColumnSpec::default(),
                                        {
                                            let post = post.clone();
                                            move || {
                                                Text(
                                                    format!("Post #{}", post.id),
                                                    Modifier::empty()
                                                        .padding(6.0)
                                                        .background(Color(0.3, 0.5, 0.8, 0.6))
                                                        .rounded_corners(8.0),
                                                );

                                                Spacer(Size {
                                                    width: 0.0,
                                                    height: 8.0,
                                                });

                                                Text(
                                                    format!("Title: {}", post.title),
                                                    Modifier::empty().padding(4.0),
                                                );

                                                Spacer(Size {
                                                    width: 0.0,
                                                    height: 4.0,
                                                });

                                                Text(
                                                    format!("User ID: {}", post.user_id),
                                                    Modifier::empty()
                                                        .padding(4.0)
                                                        .background(Color(0.2, 0.2, 0.3, 0.4))
                                                        .rounded_corners(6.0),
                                                );
                                            }
                                        },
                                    );
                                }
                            }
                        },
                    );
                }
                FetchState::Error(ref error) => {
                    let error_msg = error.clone();
                    Column(
                        Modifier::empty()
                            .padding(16.0)
                            .background(Color(0.4, 0.2, 0.2, 0.7))
                            .rounded_corners(14.0),
                        ColumnSpec::default(),
                        move || {
                            Text(
                                "Error!",
                                Modifier::empty()
                                    .padding(8.0)
                                    .background(Color(0.8, 0.3, 0.3, 0.8))
                                    .rounded_corners(10.0),
                            );

                            Spacer(Size {
                                width: 0.0,
                                height: 8.0,
                            });

                            Text(error_msg.clone(), Modifier::empty().padding(4.0));
                        },
                    );
                }
            }
        },
    );
}

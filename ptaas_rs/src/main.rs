use ptaas_rs::models::{
    APIGerneralResponseErrorType, APIResponse, APIResponseError, APIResponseType,
    AllProjectsResponseData, AllProjectsResponseErrorType, Project, Script,
};

fn main() {
    // Create fake data for an API response indicating success
    let projects = vec![
        Project {
            id: String::from("project1"),
            installed: true,
            scripts: vec![
                Script {
                    id: String::from("script1"),
                },
                Script {
                    id: String::from("script2"),
                },
            ],
        },
        Project {
            id: String::from("project2"),
            installed: false,
            scripts: vec![Script {
                id: String::from("script3"),
            }],
        },
    ];

    let all_projects_data = AllProjectsResponseData { projects };

    let api_response: APIResponse<_, ()> = APIResponse {
        response_type: APIResponseType::AllProjectsResponse,
        data: Some(all_projects_data),
        error: None,
    };

    println!(
        "API Response:\n{}\n",
        serde_json::to_string(&api_response).unwrap()
    );

    // Create fake data for an API response indicating an error
    let error_type = AllProjectsResponseErrorType::CantReadProjects;
    let error_message = String::from("Failed to read projects.");

    let error = APIResponseError {
        error_type,
        error_message,
    };

    let api_error_response: APIResponse<(), _> = APIResponse {
        response_type: APIResponseType::AllProjectsResponse,
        data: None,
        error: Some(error),
    };

    println!(
        "API Error Response:\n{}\n",
        serde_json::to_string(&api_error_response).unwrap()
    );

    // Create an unsuccessful APIGeneralResponse with an error
    let error_type = APIGerneralResponseErrorType::APIKeyIsMissing;
    let error_message = String::from("API key is missing.");

    let error = APIResponseError {
        error_type,
        error_message,
    };

    let api_error_response: APIResponse<(), _> = APIResponse {
        response_type: APIResponseType::GerneralResponse,
        data: None,
        error: Some(error),
    };

    println!(
        "API General Error Response: \n{}",
        serde_json::to_string(&api_error_response).unwrap()
    );
}

use std::fmt::Display;
use std::io;
use walkdir::WalkDir;
use std::io::ErrorKind;

use super::request::Version;
use super::request::HttpRequest;

#[derive(Debug)]
pub struct HttpResponse {
    version: Version,
    status: ResponseStatus,
    content_length: usize,
    accept_ranges: AcceptRanges,
    pub response_body: String,
    pub current_path: String,
}

impl HttpResponse {
    pub fn new(request: &HttpRequest) -> io::Result<HttpResponse> {
        let version: Version = Version::V1_1;
        let mut status: ResponseStatus = ResponseStatus::NotFound;
        let mut content_length: usize = 0;
        let mut accept_ranges: AcceptRanges = AcceptRanges::None;
        let mut response_body = String::new();

        let server_root_path = std::env::current_dir()?; // Get current working directory (root of the server)
        let resource = if request.resource.path.is_empty() || request.resource.path == "/" {
            ".".to_string() // Represent the current directory if no resource path is specified
        } else {
            request.resource.path.clone() // Get the requested path
        };
        let new_path = server_root_path.join(&resource);        

        // Log the requested path
        println!("Requested path: {:?}", new_path);

        let rootcwd_len = server_root_path.canonicalize()?.components().count();
        let resource_len = new_path.canonicalize()?.components().count();

        // Check if path is within the server root
        if rootcwd_len > resource_len {
            status = ResponseStatus::NotFound;
            response_body = "<html><body><h1>403 Forbidden</h1></body></html>".to_string();
            content_length = response_body.len();
            println!("403 Response: {}", response_body);
        } 
        // Check if the requested path exists
        else if new_path.exists() {
            // If it's a file, serve the file content
            if new_path.is_file() {
                println!("Serving file: {:?}", new_path);
                
                let content = std::fs::read(&new_path)?;  // Read the file content
                content_length = content.len();
                status = ResponseStatus::OK;
                accept_ranges = AcceptRanges::Bytes;

                // Infer the MIME type
                let mime_type = infer::get_from_path(&new_path)?
                    .map_or("application/octet-stream", |t| t.mime_type());

                response_body = format!(
                    "{} {}\n{}\nContent-Type: {}\nContent-Length: {}\r\n\r\n",
                    version, status, accept_ranges, mime_type, content_length
                );
                response_body.push_str(&String::from_utf8_lossy(&content));  // Append the file content
                
                println!("Response Body for File: {}", response_body);
            } 
            // If it's a directory, generate a directory listing
            else if new_path.is_dir() {
                println!("Serving directory: {:?}", new_path);

                let mut dir_list = String::new(); // Prepare directory list

                // Add "Go Back" button unless we're at the root directory
                if resource != "." {
                    let parent_path = std::path::Path::new(&resource).parent().unwrap_or_else(|| std::path::Path::new("/")).display().to_string();
                    dir_list.push_str(&format!(
                        "<li><a href=\"/{}\">Go back up a directory</a></li>", 
                        parent_path
                    ));
                }

                // Iterate through the directory and collect file/folder names
                for entry in WalkDir::new(&new_path).min_depth(1).max_depth(1) {
                    let entry = entry?;
                    let file_name = entry.file_name().to_string_lossy();

                    // Generate the file path relative to the server root for correct linking
                    let file_path = match entry.path().strip_prefix(&server_root_path) {
                        Ok(path) => path.display().to_string(),
                        Err(_) => return Err(std::io::Error::new(ErrorKind::Other, "Failed to strip prefix")),
                    };

                    // Create clickable links for files/folders
                    dir_list.push_str(&format!(
                        "<li><a href=\"/{}\">{}</a></li>", 
                        file_path, file_name
                    ));
                }

                // Build the HTML body with the directory listing and include inline CSS for styling
                response_body = format!(
                    "<html><head><style>
                    body {{ font-family: Arial, sans-serif; margin: 20px; padding: 0; }}
                    h1 {{ color: #333; }}
                    ul {{ list-style-type: none; padding: 0; }}
                    li {{ margin-bottom: 10px; }}
                    a {{ text-decoration: none; color: #007bff; font-size: 16px; }}
                    a:hover {{ text-decoration: underline; color: #0056b3; }}
                    </style></head><body>
                    <h1>Directory Listing</h1>
                    <ul>{}</ul></body></html>", 
                    dir_list
                );

                // Calculate content length after the response body is fully generated
                content_length = response_body.len();

                // Include headers in the response body
                response_body = format!(
                    "{} {}\n{}\nContent-Type: text/html\nContent-Length: {}\r\n\r\n{}", 
                    version, status, accept_ranges, content_length, response_body
                );

                status = ResponseStatus::OK;

                println!("Response Body for Directory: {}", response_body);
            }
        } 
        // Handle case when the resource doesn't exist (404 Not Found)
        else {
            response_body = "<html><body><h1>404 Not Found</h1></body></html>".to_string();
            content_length = response_body.len();
            println!("404 Response: {}", response_body);
        }

        // Return the constructed HTTP response
        Ok(HttpResponse {
            version,
            status,
            content_length,
            accept_ranges,
            response_body,
            current_path: request.resource.path.clone(),
        })
    }
}

// Enum to represent HTTP response status codes
#[derive(Debug)]
enum ResponseStatus {
    OK = 200,
    NotFound = 404,
}

impl Display for ResponseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ResponseStatus::OK => "200 OK",
            ResponseStatus::NotFound => "404 NOT FOUND",
        };
        write!(f, "{}", msg)
    }
}

// Enum to represent Accept-Ranges header
#[derive(Debug)]
enum AcceptRanges {
    Bytes,
    None,
}

impl Display for AcceptRanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            AcceptRanges::Bytes => "accept-ranges: bytes",
            AcceptRanges::None => "accept-ranges: none",
        };
        write!(f, "{}", msg)
    }
}

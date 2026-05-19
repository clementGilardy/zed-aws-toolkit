use serde_json::{json, Value};

pub fn all_tools() -> Value {
    json!([
        {
            "name": "list_accounts",
            "description": "List all AWS SSO accounts and roles available for the configured SSO session.",
            "inputSchema": {"type": "object", "properties": {}, "required": []}
        },
        {
            "name": "switch_account",
            "description": "Switch the active AWS account/profile used for subsequent tool calls.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "profile": {"type": "string", "description": "Profile name from list_accounts (e.g. 'my-account')"}
                },
                "required": ["profile"]
            }
        },
        {
            "name": "sso_login",
            "description": "Initiate AWS SSO login flow. Returns a URL and device code to open in the browser.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "profile": {"type": "string", "description": "Profile name to authenticate"}
                },
                "required": ["profile"]
            }
        },
        {
            "name": "s3_list_buckets",
            "description": "List all S3 buckets in the active AWS account.",
            "inputSchema": {"type": "object", "properties": {}, "required": []}
        },
        {
            "name": "s3_list_objects",
            "description": "List objects in an S3 bucket, optionally filtered by prefix.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "bucket": {"type": "string", "description": "Bucket name"},
                    "prefix": {"type": "string", "description": "Key prefix filter (optional)"},
                    "max_keys": {"type": "integer", "description": "Max results (default 1000)"}
                },
                "required": ["bucket"]
            }
        },
        {
            "name": "s3_get_object",
            "description": "Download an S3 object. Returns content as UTF-8 string or base64 for binary files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "bucket": {"type": "string", "description": "Bucket name"},
                    "key": {"type": "string", "description": "Object key"}
                },
                "required": ["bucket", "key"]
            }
        },
        {
            "name": "s3_put_object",
            "description": "Upload a text object to S3.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "bucket": {"type": "string", "description": "Bucket name"},
                    "key": {"type": "string", "description": "Object key"},
                    "body": {"type": "string", "description": "Object content (UTF-8 text)"}
                },
                "required": ["bucket", "key", "body"]
            }
        },
        {
            "name": "s3_delete_object",
            "description": "Delete an object from S3.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "bucket": {"type": "string", "description": "Bucket name"},
                    "key": {"type": "string", "description": "Object key"}
                },
                "required": ["bucket", "key"]
            }
        },
        {
            "name": "s3_presign",
            "description": "Generate a pre-signed URL for downloading an S3 object.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "bucket": {"type": "string", "description": "Bucket name"},
                    "key": {"type": "string", "description": "Object key"},
                    "expires_secs": {"type": "integer", "description": "URL expiry in seconds (default 3600)"}
                },
                "required": ["bucket", "key"]
            }
        },
        {
            "name": "lambda_list",
            "description": "List all Lambda functions in the active AWS account.",
            "inputSchema": {"type": "object", "properties": {}, "required": []}
        },
        {
            "name": "lambda_invoke",
            "description": "Invoke a Lambda function synchronously and return the response.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Function name or ARN"},
                    "payload": {"type": "object", "description": "JSON payload to send to the function (optional)"}
                },
                "required": ["name"]
            }
        },
        {
            "name": "lambda_get_logs",
            "description": "Get recent CloudWatch logs for a Lambda function.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Function name"},
                    "tail": {"type": "integer", "description": "Number of most recent log events to return (optional)"}
                },
                "required": ["name"]
            }
        },
        {
            "name": "logs_list_groups",
            "description": "List CloudWatch log groups, optionally filtered by prefix.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "prefix": {"type": "string", "description": "Log group name prefix filter (optional)"}
                },
                "required": []
            }
        },
        {
            "name": "logs_list_streams",
            "description": "List log streams in a CloudWatch log group, ordered by last event time.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "group": {"type": "string", "description": "Log group name"},
                    "limit": {"type": "integer", "description": "Max streams to return (optional)"}
                },
                "required": ["group"]
            }
        },
        {
            "name": "logs_tail",
            "description": "Tail recent log events from a CloudWatch log group or stream.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "group": {"type": "string", "description": "Log group name"},
                    "stream": {"type": "string", "description": "Log stream name (optional, tails all streams if omitted)"},
                    "since": {"type": "integer", "description": "How many seconds ago to start (default 900 = 15 min)"}
                },
                "required": ["group"]
            }
        },
        {
            "name": "logs_search",
            "description": "Search CloudWatch log group using a filter pattern.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "group": {"type": "string", "description": "Log group name"},
                    "query": {"type": "string", "description": "CloudWatch filter pattern (e.g. 'ERROR', '[level=ERROR]')"},
                    "since": {"type": "integer", "description": "How many seconds ago to search (default 3600 = 1 hr)"}
                },
                "required": ["group", "query"]
            }
        },
        {
            "name": "ecs_list_clusters",
            "description": "List all ECS cluster ARNs in the active AWS account.",
            "inputSchema": {"type": "object", "properties": {}, "required": []}
        },
        {
            "name": "ecs_list_services",
            "description": "List all ECS service ARNs in a cluster.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cluster": {"type": "string", "description": "Cluster name or ARN"}
                },
                "required": ["cluster"]
            }
        },
        {
            "name": "ecs_list_tasks",
            "description": "List running ECS task ARNs in a cluster, optionally filtered by service.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cluster": {"type": "string", "description": "Cluster name or ARN"},
                    "service": {"type": "string", "description": "Service name filter (optional)"}
                },
                "required": ["cluster"]
            }
        },
        {
            "name": "ecs_describe_task",
            "description": "Describe an ECS task: status, containers (name/image/status), log configuration from task definition.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cluster": {"type": "string", "description": "Cluster name or ARN"},
                    "task_arn": {"type": "string", "description": "Task ARN"}
                },
                "required": ["cluster", "task_arn"]
            }
        },
        {
            "name": "ecr_list_repos",
            "description": "List all ECR repositories in the active AWS account.",
            "inputSchema": {"type": "object", "properties": {}, "required": []}
        },
        {
            "name": "ecr_list_images",
            "description": "List images in an ECR repository.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo": {"type": "string", "description": "Repository name"},
                    "max": {"type": "integer", "description": "Max images to return (optional)"}
                },
                "required": ["repo"]
            }
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_returns_22_tools() {
        let tools = all_tools();
        assert_eq!(tools.as_array().unwrap().len(), 22);
    }

    #[test]
    fn every_tool_has_name_description_input_schema() {
        let tools = all_tools();
        for tool in tools.as_array().unwrap() {
            assert!(tool["name"].is_string(), "missing name: {tool}");
            assert!(tool["description"].is_string(), "missing description: {tool}");
            assert!(tool["inputSchema"].is_object(), "missing inputSchema: {tool}");
        }
    }

    #[test]
    fn required_params_are_arrays() {
        let tools = all_tools();
        for tool in tools.as_array().unwrap() {
            let schema = &tool["inputSchema"];
            assert!(schema["required"].is_array(), "required must be array: {tool}");
        }
    }
}

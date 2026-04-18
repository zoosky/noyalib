//! Error path tracking example for noyalib.
//!
//! Demonstrates the `Path` type for tracking locations within YAML structures.
//! Useful for providing detailed error messages that show exactly where
//! in the document a problem occurred.

use noyalib::Path;

fn main() {
    println!("noyalib error path tracking example\n");

    // =========================================================================
    // Example 1: Building paths step by step
    // =========================================================================
    println!("=== Example 1: Building paths ===\n");

    // Start with root path
    let root = Path::Root;
    println!("Root path: {}", root);

    // Build a path into a mapping - each step borrows from the previous
    let config = root.key("config");
    println!("config: {}", config);

    // Go deeper
    let database = config.key("database");
    println!("config.database: {}", database);

    let host = database.key("host");
    println!("config.database.host: {}", host);

    // =========================================================================
    // Example 2: Sequence indices
    // =========================================================================
    println!("\n=== Example 2: Sequence indices ===\n");

    // Path into a sequence
    let servers = root.key("servers");
    println!("servers: {}", servers);

    let first_server = servers.index(0);
    println!("servers[0]: {}", first_server);

    let server_host = first_server.key("host");
    println!("servers[0].host: {}", server_host);

    // Multiple levels of nesting - each level needs its own binding
    let clusters = root.key("clusters");
    let cluster0 = clusters.index(0);
    let nodes = cluster0.key("nodes");
    let node2 = nodes.index(2);
    let ip = node2.key("ip");
    println!("clusters[0].nodes[2].ip: {}", ip);

    // =========================================================================
    // Example 3: Simulating error reporting
    // =========================================================================
    println!("\n=== Example 3: Error reporting simulation ===\n");

    // Simulate validating a configuration structure
    struct ValidationError {
        path: String,
        message: String,
    }

    fn validate_config(path: &Path<'_>, errors: &mut Vec<ValidationError>) {
        // Simulate finding errors at various paths
        let db_path = path.key("database");
        let port_path = db_path.key("port");

        errors.push(ValidationError {
            path: port_path.to_string(),
            message: "port must be between 1 and 65535".to_string(),
        });

        let users_path = path.key("users");
        let user_path = users_path.index(2);
        let email_path = user_path.key("email");

        errors.push(ValidationError {
            path: email_path.to_string(),
            message: "invalid email format".to_string(),
        });
    }

    let mut errors = Vec::new();
    validate_config(&Path::Root, &mut errors);

    println!("Validation errors:");
    for error in &errors {
        println!("  - {}: {}", error.path, error.message);
    }

    // =========================================================================
    // Example 4: Path navigation
    // =========================================================================
    println!("\n=== Example 4: Path navigation ===\n");

    let level1 = Path::Root.key("level1");
    let level2 = level1.key("level2");
    let level2_0 = level2.index(0);
    let level3 = level2_0.key("level3");

    println!("Full path: {}", level3);

    // Get parent at each level
    if let Some(parent) = level3.parent() {
        println!("Parent 1: {}", parent);
        if let Some(grandparent) = parent.parent() {
            println!("Parent 2: {}", grandparent);
            if let Some(great_grandparent) = grandparent.parent() {
                println!("Parent 3: {}", great_grandparent);
            }
        }
    }

    // =========================================================================
    // Example 5: Alias paths
    // =========================================================================
    println!("\n=== Example 5: Alias tracking ===\n");

    // When following an alias, we can track it
    let defaults = Path::Root.key("defaults");
    println!("Anchor location: {}", defaults);

    let production = Path::Root.key("production");
    let alias_usage = production.alias();
    println!("Alias usage: {}", alias_usage);

    // =========================================================================
    // Example 6: Unknown paths
    // =========================================================================
    println!("\n=== Example 6: Unknown path elements ===\n");

    // Sometimes we don't know the key name (e.g., during streaming)
    let data = Path::Root.key("data");
    let unknown = data.unknown();
    println!("Unknown element: {}", unknown);

    // =========================================================================
    // Example 7: Practical error message formatting
    // =========================================================================
    println!("\n=== Example 7: Formatted error messages ===\n");

    fn format_error(path: &Path<'_>, expected: &str, found: &str) -> String {
        format!(
            "Type mismatch at `{}`:\n  expected: {}\n  found: {}",
            path, expected, found
        )
    }

    let services = Path::Root.key("services");
    let service0 = services.index(0);
    let port = service0.key("port");

    let error_msg = format_error(&port, "integer", "string \"http\"");
    println!("{}", error_msg);

    // =========================================================================
    // Example 8: Path comparison
    // =========================================================================
    println!("\n=== Example 8: Path comparison ===\n");

    let config1 = Path::Root.key("config");
    let port1 = config1.key("port");

    let config2 = Path::Root.key("config");
    let port2 = config2.key("port");

    let config3 = Path::Root.key("config");
    let host3 = config3.key("host");

    println!("path1: {}", port1);
    println!("path2: {}", port2);
    println!("path3: {}", host3);
    println!("path1 == path2: {}", port1 == port2);
    println!("path1 == path3: {}", port1 == host3);

    // =========================================================================
    // Example 9: Building paths in a loop
    // =========================================================================
    println!("\n=== Example 9: Building paths dynamically ===\n");

    // Demonstrate building paths when iterating over data
    let items = Path::Root.key("items");

    for i in 0..3 {
        let item = items.index(i);
        let name = item.key("name");
        println!("Item {} path: {}", i, name);
    }

    println!("\nError path tracking example completed!");
}

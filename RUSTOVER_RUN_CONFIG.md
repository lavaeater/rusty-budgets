# Running Dioxus in RustRover

This document explains how to set up run configurations in RustRover to launch and debug your Dioxus application without using the `dx serve` command.

## Understanding the `dx serve` Command

The `dx serve --package desktop` command does the following:
1. Builds the server component with the "server" feature enabled
2. Builds the client component with the "desktop" feature enabled
3. Runs both components simultaneously

## Quick Start: Using the Shell Script

For convenience, a shell script has been provided that replicates the functionality of `dx serve`:

```bash
# Make the script executable (if not already)
chmod +x run_dioxus.sh

# Run the script
./run_dioxus.sh
```

The script will:
- Start the server component
- Wait for it to initialize
- Start the client component
- Handle proper cleanup when terminated

This is the simplest way to run the application outside of using `dx serve`.

### Debugging with the Script

While the script is convenient for running the application, it doesn't provide the same debugging capabilities as RustRover's built-in debugger. For debugging:

1. You can modify the script to use `cargo run --package desktop --no-default-features --features server -- --debug` for the server component
2. However, for full debugging capabilities with breakpoints and variable inspection, use the RustRover run configurations described below

## RustRover Run Configurations

### Server Component

1. Go to "Run" → "Edit Configurations..."
2. Click the "+" button and select "Cargo"
3. Configure as follows:
   - Name: "Server"
   - Command: "run"
   - Package: "desktop"
   - Features: "server"
   - Arguments: "--no-default-features"
4. Apply and save

### Client Component

1. Go to "Run" → "Edit Configurations..."
2. Click the "+" button and select "Cargo"
3. Configure as follows:
   - Name: "Desktop Client"
   - Command: "run"
   - Package: "desktop"
   - Features: "desktop"
   - Arguments: "--no-default-features"
4. Apply and save

## Debugging

- Use the debug button instead of the run button to enable debugging
- Set breakpoints in your code as needed
- For the server component, you can set breakpoints in the API code
- For the client component, you can set breakpoints in the UI code

## Running Both Components

### Manual Launch

To replicate the behavior of `dx serve`, you'll need to:
1. Start the Server configuration first
2. Once the server is running, start the Desktop Client configuration

### Compound Run Configuration

For a more integrated experience, you can create a compound run configuration:

1. Go to "Run" → "Edit Configurations..."
2. Click the "+" button and select "Compound"
3. Name it "Dioxus Full Stack"
4. Add both the "Server" and "Desktop Client" configurations
5. Make sure "Server" is listed first
6. Check "Allow parallel run" to run both configurations simultaneously
7. Apply and save

When you run this compound configuration, RustRover will start both the server and client components in the correct order.

This setup allows you to launch and debug both components from within RustRover.
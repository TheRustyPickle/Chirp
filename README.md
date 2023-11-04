<div align="center"><h1>Chirp: Exploring GTK4-rs through Chat</h1></div>

<div align=center><a href="https://wakatime.com/badge/github/TheRustyPickle/Chirp"><img src="https://wakatime.com/badge/github/TheRustyPickle/Chirp.svg" alt="wakatime"></a></div>

Chirp is my playground for exploring the world of GTK4-rs while working on a chat application. Currently, it's a practice project with a straightforward and functional UI.

## Features

🎨 **User Interface:** Chirp features an interface crafted using GTK4-rs.

🌐 **Server:** A WebSocket server created with actix-web the GUI can communicate with, allowing usage of multiple clients with auto-reconnecting.

🛡️ **Security:** The application incorporates several security measures, including TLS-encrypted server communication and token-based authentication for the GUI client.

💬 **Messaging:** The app supports basic messaging capabilities including sending and deleting messages, adding new chat, and message synchronization upon startup.

🔒 **Message Encryption:** A combination of RSA and AES is used to add encryption to every single message and is decrypted locally to show it in the UI.

<details>
<summary>App Screenshots</summary>
  <img src="https://github.com/TheRustyPickle/Chirp/assets/35862475/d475e493-37aa-4309-a256-5ec54caefe77">
  <img src="https://github.com/TheRustyPickle/Chirp/assets/35862475/548621ca-f632-4799-931f-a8580fce672f">
</details>

## Current Status

No further development is planned for the project as it served its purpose

## Project Components

- `gui/`: Contains the UI interface built with GTK4-rs along with all the logic and UI components to make it run.
- `server/`: Hosts a WebSocket server created with actix-web, facilitating communication with the GUI and managing the DB.
- `migrations/`: Contains DB migrations details, should be handled with diesel-rs

## Explore the Project

- Clone this project onto your local machine `git clone https://github.com/TheRustyPickle/Chirp.git`.
- Ensure you have the required dependencies, including the latest GTK4, Libadwaita libraries, and Postgres.
- Install diesel cli `cargo install diesel_cli`
- Update Postgres credentials on `.env` file
- Setup DB and run migrations

```bash
diesel setup
diesel migration run
```

- Setup GTK Schema settings

```bash
mkdir -p $HOME/.local/share/glib-2.0/schemas
cp ./gui/src/com.github.therustypickle.chirp.gschema.xml $HOME/.local/share/glib-2.0/schemas/
glib-compile-schemas $HOME/.local/share/glib-2.0/schemas/
```

- Start the server `cargo run --bin chirp-server --release`
- Launch the GUI using the command `cargo run --bin chirp-gui --release`

## License

Chirp is under the [MIT License](LICENSE).

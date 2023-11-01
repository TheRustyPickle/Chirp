<div align="center"><h1>Chirp: Exploring GTK4-rs through Chat</h1></div>

<div align=center><a href="https://wakatime.com/badge/github/TheRustyPickle/Chirp"><img src="https://wakatime.com/badge/github/TheRustyPickle/Chirp.svg" alt="wakatime"></a></div>

Chirp is my playground for exploring the world of GTK4-rs while working on a chat application. Currently, it's a practice project with a straightforward and functional UI.

## Current Status

🎨 **User Interface:** Chirp features an interface crafted using GTK4-rs. It is being actively developed and refined with more features.

🌐 **Server:** A WebSocket server created with actix-web the GUI can communicate with, allowing usage of multiple clients with auto-reconnecting.

🛡️ **Security:** The application incorporates several security measures, including TLS-encrypted server communication and token-based authentication for the GUI client.

💬 **Messaging:** The app supports basic messaging capabilities including sending and deleting messages, adding new chat, and message synchronization upon startup.

<details>
<summary>App Screenshots</summary>
  <img src="https://github.com/TheRustyPickle/Chirp/assets/35862475/ad9ef82e-dc2f-40b9-8fa7-0df20a3dc62e">
  <img src="https://github.com/TheRustyPickle/Chirp/assets/35862475/5f7b22c1-3afd-44f9-928b-dfabc2ffd236">
</details>

## What's on the Horizon?

🔧 **Refining UI:** Further refining of the user interface for a more user-friendly experience and with more features.

🔒 **Message Encryption:** Implementation of encryption measures to enhance the privacy and security of messages.

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

## Get Involved

Want to contribute or share ideas? All participation is welcome! Feel free to open an issue or submit a pull request to get the conversation going.

## License

Chirp is under the [MIT License](LICENSE).

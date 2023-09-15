<div align="center"><h1>Chirp: Exploring GTK4-rs through Chat</h1></div>

<div align=center><a href="https://wakatime.com/badge/github/TheRustyPickle/Chirp"><img src="https://wakatime.com/badge/github/TheRustyPickle/Chirp.svg" alt="wakatime"></a></div>

Chirp is my playground for exploring the world of GTK4-rs while working on a chat application. Currently, it's a practice project with a straightforward and functional UI.

## What's in the Mix?

🎨 **User Interface (UI):** Chirp features an interface crafted using GTK4-rs. I'm refining it into a chat app that offers a practical and smooth experience, where messages are composed and displayed effortlessly.

🌐 **WebSocket Support:** Chirp now contains a simple WebSocket server created with actix-web the GUI can communicate with, allowing usage of multiple clients with auto-reconnecting.

<details>
<summary>App Screenshots</summary>
  <img src="https://github.com/TheRustyPickle/Chirp/assets/35862475/a869093b-ea83-4fb0-9111-31e13f4ac64b">
  <img src="https://github.com/TheRustyPickle/Chirp/assets/35862475/928f96e4-72f1-49ac-a550-b82e843095a9">
</details>

## What's on the Horizon?

🔧 **Refining UI:** Further refining the user interface for a more user-friendly experience.

🛡️ **Security:** Enhancing overall security, especially WebSocket communication and authentication.

🔒 **Message Encryption:** Implementation of encryption measures to further enhance the privacy and security of messages.

## Project Components

- `gui/`: Contains the UI interface built with GTK4-rs along with all the logic and components to make it run.
- `server/`: Hosts a simple WebSocket server created with actix-web, facilitating communication with the GUI.

## Exploring the Project

1. Clone this project onto your local machine `git clone https://github.com/TheRustyPickle/Chirp.git`.
2. Ensure that you have the required dependencies, including GTK4 libraries.
3. Start the WebSocket server using the command `cargo run --bin chirp-server`
4. Launch the GUI using the command `cargo run --bin chirp-gui`

## Get Involved

Want to contribute or share ideas? Any participation is welcome! Feel free to open an issue or submit a pull request to get the conversation going.

## Licensing

Chirp is under the [MIT License](LICENSE).

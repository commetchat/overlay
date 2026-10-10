
# Overlay

<p align="left">
    <a href="https://commet.chat/donate"><img alt="Donate" src="https://img.shields.io/badge/donate-534cdd?style=for-the-badge"></a>
    <a href="https://matrix.to/#/#commet:matrix.org"><img alt="Matrix" src="https://img.shields.io/matrix/commet%3Amatrix.org?logo=matrix&style=for-the-badge&color=534cdd"></a>
    <a href="https://fosstodon.org/@commetchat"><img alt="Mastodon" src="https://img.shields.io/mastodon/follow/109894490854601533?domain=https%3A%2F%2Ffosstodon.org&style=for-the-badge&logo=mastodon&color=534cdd&logoColor=white"></a>
    <a href="https://bsky.app/profile/commet.chat"><img alt="Bluesky" src="https://img.shields.io/badge/follow-@commet.chat-whitesmoke?style=for-the-badge&logo=bluesky&logoColor=white&color=534cdd"></a>
</p>

### Voice Chat Overlay for Commet

A simple screen overlay for displaying the state of voice calls above other windows. 

Supports Windows and Linux (Both x11, and wayland)

Written in Rust, it aims to be as lightweight as possible only rendering when the voice chat state changes, and should consume only ~30mb of RAM.

### Running
To test the overlay with some dummy data:
```
cargo run
```

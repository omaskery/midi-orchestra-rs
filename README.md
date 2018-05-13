# Overview

This is just some silly software for playing midi files across several machines at once, forming a "MIDI orchestra". I originally wrote a version of this in python a few years ago, but recently had an urge to reimplement it in Rust!

The application runs in two modes: server and client. The server at minimum takes a path to a MIDI file as input, whilst the client at minimum takes a hostname:port-number target string to connect to. The server delays briefly to allow clients to connect before beginning playback, distributing notes to connections on the fly. Clients connect to and awaits commands from the server.

If using headphones I advise passing the --volume parameter to the server to avoid ear destruction. When invoking clients I advise passing the --forever parameter, save launching the client repeatedly after each song.

In the event that playback isn't very pleasing it can sometimes be alleviated by disabling certain MIDI tracks or channels. This can be controlled with the --exclude-track and --exclude-channel arguments. In order to discover which channels/tracks to exclude it can be worth listening to single channels with --include-channel and single tracks with --include-track.

MIDI conventionally treats channel 10 as percussion which this software doesn't handle very well (partially omitted by design) - the software therefore automatically disables channel 10. To inhibit this behaviour pass --allow-channel-10.

# Examples

## Running the server

Simply running the server:

`midi-orchestra-rs server path/to/music.mid`

Specifying a volume:

`midi-orchestra-rs server path/to/music.mid --volume=0.5`

Excluding a specific channel:

`midi-orchestra-rs server path/to/music.mid --exclude-channel 3`

Excluding a specific track:

`midi-orchestra-rs server path/to/music.mid --exclude-track 5`

Playing **only** a specific channel:

`midi-orchestra-rs server path/to/music.mid --include-channel 3`

Playing **only** a specific track:

`midi-orchestra-rs server path/to/music.mid --include-track 5`

## Running a client

Simply running a client:

`midi-orchestra-rs client localhost:4000`

Running a client 'forever' to avoid having to invoke it after each song:

`midi-orchestra-rs client localhost:4000 --forever`

# To do

- [x] refactor so that midi code has tempo handling baked in, that way the subsequent systems can just manipulate musical events without constantly juggling time concerns
- [ ] then, based on that, can use the include/exclude track/channel arguments to simply filter what events go forward into the actual playback loop
- [ ] once that's done, look into assigning frequency ranges to clients by placing all notes into a histogram (x-axis = frequency) and giving each client an equal "area under the graph" of adjacent buckets, which should now be possible since all musical events (after filtering) are now available
- [ ] make channel/track filtering collect<>() using hashset or something rather than doing all that contains() checking with a Vec
- [ ] make channel/track filter arguments build hashset of legal tracks/channels and simply check those during note playing rather than obtuse logic around each list being present etc.

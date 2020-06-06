# MIDI Tool

A little tool to help with typical MIDI problems.

With an increasing number of devices offering MIDI over USB, and thus showing
up as a dedicated input/ output MIDI port in the system, it's sometimes
overkill to have to fire up a DAW just to route an input device to the right
target. This utility aims to help with that.

It supports:

- Listing the active MIDI ports of the system
- Forward data from one or more MIDI input ports to one or more MIDI output ports
- Change the MIDI channel of a message
- Monitor the received data
- Write the received data to a file

A single source and destination port can be given as command line parameters.
For more complex scenarios, the configuration can be read from a CSV file.

## Some examples

Forward MIDI data from port 1 to port 2:

    miditool -i 1 -o 2

Forward data from port 1 channel 1 to port 2, change the channel to 3, print the data:

    miditool -i 1 -c 1 -o 2 -n 3 -m

Forward all data from port 1 and port 2 to port 3. This reads the
configuration from a file config.csv, which has the following content:

    1,0,3,0
    2,0,3,0

The columns are in-port, in-channel (0 for omni), out-port, out-channel.

    miditool -d config.csv

Write data from port 1 to a file:

    miditool -i 1 -w output

This will create the file output_p1. When reading from multiple ports, each
port will get it's own output file.

## Planned functionality:

- Replay previously recorded MIDI data from a file
- Send MIDI files

# rust-pi-robot

## Synopsys

```rust
extern crate pi_robot;

let mut robot = pi_robot::Robot::new("robot.yaml").unwrap()
```

## Sound Issues

For warnings like:

```
ALSA lib pcm.c:2495:(snd_pcm_open_noupdate) Unknown PCM cards.pcm.modem
ALSA lib pcm.c:2495:(snd_pcm_open_noupdate) Unknown PCM cards.pcm.phoneline
```

edit /usr/share/alsa/alsa.conf and set channels to the default like

```
pcm.front cards.pcm.default
```

//! songs: metadata, notes, layers, and custom instruments.

use crate::note::{Note, NoteStreamExt, Notes};
use crate::types::{Index, Panning, Position, Result, Tick, Version, Volume};

/// represents a complete nbs song with header, notes, layers, and instruments.
#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
pub struct Song {
    /// song metadata read from the nbs header.
    pub header: Header,
    // Position data is stored redundantly due to incremental encoding
    // I don't know why the header's song_length is only u16 :(
    /// every note, keyed by its tick and layer.
    pub notes: Notes<Position, Note>,
    /// layers in index order; a note's layer index selects one of these.
    pub layers: Vec<Layer>,
    /// custom instruments in index order, referenced by
    /// [`Instrument::Custom`](crate::note::Instrument::Custom).
    pub custom_instruments: Vec<CustomInstrument>,
}

impl Song {
    /// creates a new empty song with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// opens and parses an nbs file from a file path
    pub fn open_nbs<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let mut file = std::fs::File::open(path)?;
        Self::parse(&mut file)
    }

    /// parses an nbs file from standard input
    pub fn from_stdin() -> Result<Self> {
        let mut stdin = std::io::stdin();
        Self::parse(&mut stdin)
    }

    /// saves the song to an nbs file at the specified path
    pub fn save_nbs<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        let mut file = std::fs::File::create(path)?;
        self.write(&mut file)
    }

    /// writes the song to standard output
    pub fn to_stdout(&mut self) -> Result<()> {
        let mut stdout = std::io::stdout();
        self.write(&mut stdout)
    }

    /// Resolves the song's notes against the layers they sit on, yielding
    /// how each will actually sound.
    pub fn resolved_notes(self) -> impl Iterator<Item = (Position, Note)> {
        let Self { notes, layers, .. } = self;
        notes.apply_layers(layers)
    }

    /// returns the tick count (max tick + 1) of the song.
    pub fn len(&self) -> Tick {
        self.notes.tick_count()
    }
}

// header
//
// ++++++++++++============++++++++++++============++++++++++++============

/// contains metadata and song information from the nbs file header.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Header {
    /// nbs file format version the song was written in.
    pub version: Version,
    /// number of vanilla Minecraft instruments available in the song
    /// (`10`, `16`, or `20`).
    pub default_instruments: u8,
    /// song length in ticks, as stored in the header; Note Block Studio
    /// derives it from the last note's tick when saving.
    pub song_length: Tick,
    /// number of layers the song had when it was last saved.
    pub song_layers: Index,
    /// title of the song.
    pub song_name: String,
    /// author of the song.
    pub song_author: String,
    /// author of the piece this song is based on, if any.
    pub original_author: String,
    /// free-form description or notes about the song.
    pub description: String,
    /// tempo in ticks per second, stored with two decimal places; Note Block
    /// Studio defaults to `10`.
    pub tempo: f32,
    /// whether autosave was enabled in Note Block Studio.
    pub auto_save: bool,
    /// autosave interval in minutes.
    pub auto_save_duration: u32,
    /// time signature numerator, i.e. beats per bar.
    pub time_signature: u8,
    /// total minutes spent editing the song, per Note Block Studio's counter.
    pub minutes_spent: u32,
    /// left mouse clicks made while editing.
    pub left_clicks: u32,
    /// right mouse clicks made while editing.
    pub right_clicks: u32,
    /// note blocks added while editing.
    pub blocks_added: u32,
    /// note blocks removed while editing.
    pub blocks_removed: u32,
    /// file name of the original MIDI or schematic the song comes from.
    pub song_origin: String,
    /// whether playback should loop.
    pub is_loop: bool,
    /// number of times to loop; `0` means loop forever.
    pub max_loop_count: u32,
    /// tick to jump back to when looping.
    pub loop_start: Tick,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            version: Version::default(),
            default_instruments: Version::default().vanilla_instruments(),
            song_length: 0,
            song_layers: 0,
            song_name: String::new(),
            song_author: String::new(),
            original_author: String::new(),
            description: String::new(),
            tempo: 10.0,
            auto_save: false,
            auto_save_duration: 10,
            time_signature: 4,
            minutes_spent: 0,
            left_clicks: 0,
            right_clicks: 0,
            blocks_added: 0,
            blocks_removed: 0,
            song_origin: String::new(),
            is_loop: false,
            max_loop_count: 0,
            loop_start: 0,
        }
    }
}

// layer
//
// ++++++++++++============++++++++++++============++++++++++++============

/// represents a layer with a name, volume, panning, and a disabled flag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Layer {
    /// display name of the layer.
    pub name: String,
    /// whether the layer is locked; locked layers are skipped during
    /// playback.
    pub disabled: bool,
    /// layer-wide linear amplitude gain, multiplied with each note's velocity.
    pub volume: Volume,
    /// layer-wide stereo position, combined with each note's panning.
    pub panning: Panning,
}

impl Layer {
    /// A neutral layer: unnamed, enabled, full volume, centered panning.
    pub const fn new() -> Self {
        Self {
            name: String::new(),
            disabled: false,
            volume: Volume::FULL,
            panning: Panning::CENTER,
        }
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new()
    }
}

// custom instrument
//
// ++++++++++++============++++++++++++============++++++++++++============

/// represents an instrument with sound file, pitch, and playback settings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomInstrument {
    /// display name of the instrument.
    pub name: String,
    /// sound file name or path, such as `custom.ogg`.
    pub file: String,
    /// key the sound file is tuned to, in `0..=87`; Note Block Studio
    /// defaults to `45` (`F#4`).
    pub pitch: u8,
    /// whether the piano preview automatically presses the matching key
    /// while the song plays.
    pub press_key: bool,
}

impl Default for CustomInstrument {
    fn default() -> Self {
        Self {
            name: String::new(),
            file: String::new(),
            pitch: 45,
            press_key: true,
        }
    }
}

extern crate paste;

macro_rules! create_iterable_struct {
    ($struct_name:ident, [$(($field:ident, $type:ty, $version:expr)),*]) => {
        // Define the struct
        pub struct $struct_name {
            $(
                $field: Option<$type>,
            )*
        }

        // Define an enum to hold references to the fields
        paste::item! {
            enum [<$struct_name FieldRef>]<'a> {
                $(
                    [< $field:camel >](&'a Option<$type>, i8),
                )*
            }
        }

        // Implement the iterator function for the struct
        impl $struct_name {
            paste::item! {
                fn iter(&self) -> impl Iterator<Item = [<$struct_name FieldRef>]<'_>> {
                    vec![
                        $(
                            [<$struct_name FieldRef>]::[< $field:camel >](&self.$field, $version),
                        )*
                    ].into_iter()
                }
            }
        }
    };
}

create_iterable_struct!(
    NoteBlockSongHeader,
    [
        (version, i8, 1),
        (default_instruments, i8, 1),
        (song_length, i16, 3),
        (song_layers, i16, 0),
        (song_name, String, 0),
        (song_author, String, 0),
        (original_author, String, 0),
        (description, String, 0),
        (tempo, i16, 0),
        (auto_save, bool, 0),
        (auto_saving_duration, i8, 0),
        (time_signature, i8, 0),
        (minutes_spent, i32, 0),
        (left_clicks, i32, 0),
        (right_clicks, i32, 0),
        (blocks_added, i32, 0),
        (blocks_removed, i32, 0),
        (og_file, String, 0),
        (r#loop, bool, 4),
        (max_loop_count, i8, 4),
        (loop_start, i16, 4)
    ]
);

create_iterable_struct!(
    NoteBlockSongNote,
    [
        (tick, i16, 0),
        (layer, i16, 0),
        (instrument, i8, 0),
        (key, i8, 0),
        (velocity, i8, 4),
        (panning, u8, 4),
        (pitch, i16, 4)
    ]
);

create_iterable_struct!(
    NoteBlockSongLayer,
    [
        (id, i16, 127), // don't read/write only set as struct property
        (name, String, 0),
        (lock, bool, 4),
        (volume, i8, 0),
        (stereo, u8, 2)
    ]
);

create_iterable_struct!(
    NoteBlockSongInstrument,
    [
        (id, i16, 127),
        (name, String, 0),
        (file, String, 0),
        (pitch, i8, 0),
        (press_key, bool, 0)
    ]
);

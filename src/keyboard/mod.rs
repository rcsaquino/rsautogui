//! # Keyboard controls
//!
//! `keyboard` is a collection of functions used to control the keyboard.
//!
//! [`Key`] uses Windows virtual-key numbers as discriminants, so existing
//! `Vk::…` code keeps working — `Vk` is kept as a type alias for [`Key`].
//! The same keys are mapped to native equivalents on Linux (X11 keysyms)
//! and macOS (CG keycodes).

use crate::platform as plat;

/// A keyboard (or mouse-button-as-key, for compatibility) code.
///
/// Discriminants are Windows `VK_*` numbers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    MouseLeft = 1,
    MouseRight = 2,
    Cancel = 3,
    MouseMiddle = 4,
    MouseX1 = 5,
    MouseX2 = 6,
    Backspace = 8,
    Tab = 9,
    Clear = 12,
    Enter = 13,
    Shift = 16,
    Control = 17,
    Alt = 18,
    Pause = 19,
    CapsLock = 20,
    Kana = 21,
    ImeOn = 22,
    Junja = 23,
    Final = 24,
    Kanji = 25,
    ImeOff = 26,
    Escape = 27,
    Convert = 28,
    NonConvert = 29,
    Accept = 30,
    ModeChange = 31,
    Space = 32,
    PageUp = 33,
    PageDown = 34,
    End = 35,
    Home = 36,
    LeftArrow = 37,
    UpArrow = 38,
    RightArrow = 39,
    DownArrow = 40,
    Select = 41,
    Print = 42,
    Execute = 43,
    PrintScreen = 44,
    Insert = 45,
    Delete = 46,
    Help = 47,
    _0 = 48,
    _1 = 49,
    _2 = 50,
    _3 = 51,
    _4 = 52,
    _5 = 53,
    _6 = 54,
    _7 = 55,
    _8 = 56,
    _9 = 57,
    A = 65,
    B = 66,
    C = 67,
    D = 68,
    E = 69,
    F = 70,
    G = 71,
    H = 72,
    I = 73,
    J = 74,
    K = 75,
    L = 76,
    M = 77,
    N = 78,
    O = 79,
    P = 80,
    Q = 81,
    R = 82,
    S = 83,
    T = 84,
    U = 85,
    V = 86,
    W = 87,
    X = 88,
    Y = 89,
    Z = 90,
    LeftWin = 91,
    RightWin = 92,
    Apps = 93,
    Sleep = 95,
    Numpad0 = 96,
    Numpad1 = 97,
    Numpad2 = 98,
    Numpad3 = 99,
    Numpad4 = 100,
    Numpad5 = 101,
    Numpad6 = 102,
    Numpad7 = 103,
    Numpad8 = 104,
    Numpad9 = 105,
    Multiply = 106,
    Add = 107,
    Separator = 108,
    Subtract = 109,
    Decimal = 110,
    Divide = 111,
    F1 = 112,
    F2 = 113,
    F3 = 114,
    F4 = 115,
    F5 = 116,
    F6 = 117,
    F7 = 118,
    F8 = 119,
    F9 = 120,
    F10 = 121,
    F11 = 122,
    F12 = 123,
    F13 = 124,
    F14 = 125,
    F15 = 126,
    F16 = 127,
    F17 = 128,
    F18 = 129,
    F19 = 130,
    F20 = 131,
    F21 = 132,
    F22 = 133,
    F23 = 134,
    F24 = 135,
    Numlock = 144,
    Scroll = 145,
    LeftShift = 160,
    RightShift = 161,
    LeftControl = 162,
    RightControl = 163,
    LeftMenu = 164,
    RightMenu = 165,
    BrowserBack = 166,
    BrowserForward = 167,
    BrowserRefresh = 168,
    BrowserStop = 169,
    BrowserSearch = 170,
    BrowserFavorites = 171,
    BrowserHome = 172,
    VolumeMute = 173,
    VolumeDown = 174,
    VolumeUp = 175,
    NextTrack = 176,
    PrevTrack = 177,
    MediaStop = 178,
    MediaPlayPause = 179,
    StartMail = 180,
    SelectMedia = 181,
    StartApp1 = 182,
    StartApp2 = 183,
    Oem1 = 186,
    Plus = 187,
    Comma = 188,
    Minus = 189,
    Period = 190,
    Oem2 = 191,
    Oem3 = 192,
    Oem4 = 219,
    Oem5 = 220,
    Oem6 = 221,
    Oem7 = 222,
    Oem8 = 223,
    Oem102 = 226,
    ImeProcess = 229,
    Attn = 246,
    CrSel = 247,
    ExSel = 248,
    EraseEof = 249,
    Play = 250,
    Zoom = 251,
    Pa1 = 253,
    OemClear = 254,
}

/// Backwards-compatible alias: `Vk::A`, `Vk::Shift`, … keep working.
pub type Vk = Key;

impl Key {
    /// Creates a key from its numeric code.
    ///
    /// Kept `unsafe` for source compatibility with `winput::Vk::from_u8`;
    /// invalid codes panic instead of causing undefined behaviour.
    ///
    /// # Safety
    /// Safe to call with any `u8`; codes without a corresponding variant
    /// panic instead of invoking undefined behaviour.
    pub unsafe fn from_u8(n: u8) -> Self {
        Self::try_from_u8(n).expect("invalid virtual-key code")
    }

    /// Converts this key to its numeric code.
    pub fn into_u8(self) -> u8 {
        self as u8
    }

    /// Converts this key to its numeric code (wider variant).
    pub fn code(self) -> u16 {
        self as u16
    }

    /// Tries to convert a numeric code into a key, returning `None` when
    /// the code is not a known variant.
    pub fn try_from_u8(n: u8) -> Option<Self> {
        match n {
            1 => Some(Key::MouseLeft),
            2 => Some(Key::MouseRight),
            3 => Some(Key::Cancel),
            4 => Some(Key::MouseMiddle),
            5 => Some(Key::MouseX1),
            6 => Some(Key::MouseX2),
            8 => Some(Key::Backspace),
            9 => Some(Key::Tab),
            12 => Some(Key::Clear),
            13 => Some(Key::Enter),
            16 => Some(Key::Shift),
            17 => Some(Key::Control),
            18 => Some(Key::Alt),
            19 => Some(Key::Pause),
            20 => Some(Key::CapsLock),
            21 => Some(Key::Kana),
            22 => Some(Key::ImeOn),
            23 => Some(Key::Junja),
            24 => Some(Key::Final),
            25 => Some(Key::Kanji),
            26 => Some(Key::ImeOff),
            27 => Some(Key::Escape),
            28 => Some(Key::Convert),
            29 => Some(Key::NonConvert),
            30 => Some(Key::Accept),
            31 => Some(Key::ModeChange),
            32 => Some(Key::Space),
            33 => Some(Key::PageUp),
            34 => Some(Key::PageDown),
            35 => Some(Key::End),
            36 => Some(Key::Home),
            37 => Some(Key::LeftArrow),
            38 => Some(Key::UpArrow),
            39 => Some(Key::RightArrow),
            40 => Some(Key::DownArrow),
            41 => Some(Key::Select),
            42 => Some(Key::Print),
            43 => Some(Key::Execute),
            44 => Some(Key::PrintScreen),
            45 => Some(Key::Insert),
            46 => Some(Key::Delete),
            47 => Some(Key::Help),
            48 => Some(Key::_0),
            49 => Some(Key::_1),
            50 => Some(Key::_2),
            51 => Some(Key::_3),
            52 => Some(Key::_4),
            53 => Some(Key::_5),
            54 => Some(Key::_6),
            55 => Some(Key::_7),
            56 => Some(Key::_8),
            57 => Some(Key::_9),
            65 => Some(Key::A),
            66 => Some(Key::B),
            67 => Some(Key::C),
            68 => Some(Key::D),
            69 => Some(Key::E),
            70 => Some(Key::F),
            71 => Some(Key::G),
            72 => Some(Key::H),
            73 => Some(Key::I),
            74 => Some(Key::J),
            75 => Some(Key::K),
            76 => Some(Key::L),
            77 => Some(Key::M),
            78 => Some(Key::N),
            79 => Some(Key::O),
            80 => Some(Key::P),
            81 => Some(Key::Q),
            82 => Some(Key::R),
            83 => Some(Key::S),
            84 => Some(Key::T),
            85 => Some(Key::U),
            86 => Some(Key::V),
            87 => Some(Key::W),
            88 => Some(Key::X),
            89 => Some(Key::Y),
            90 => Some(Key::Z),
            91 => Some(Key::LeftWin),
            92 => Some(Key::RightWin),
            93 => Some(Key::Apps),
            95 => Some(Key::Sleep),
            96 => Some(Key::Numpad0),
            97 => Some(Key::Numpad1),
            98 => Some(Key::Numpad2),
            99 => Some(Key::Numpad3),
            100 => Some(Key::Numpad4),
            101 => Some(Key::Numpad5),
            102 => Some(Key::Numpad6),
            103 => Some(Key::Numpad7),
            104 => Some(Key::Numpad8),
            105 => Some(Key::Numpad9),
            106 => Some(Key::Multiply),
            107 => Some(Key::Add),
            108 => Some(Key::Separator),
            109 => Some(Key::Subtract),
            110 => Some(Key::Decimal),
            111 => Some(Key::Divide),
            112 => Some(Key::F1),
            113 => Some(Key::F2),
            114 => Some(Key::F3),
            115 => Some(Key::F4),
            116 => Some(Key::F5),
            117 => Some(Key::F6),
            118 => Some(Key::F7),
            119 => Some(Key::F8),
            120 => Some(Key::F9),
            121 => Some(Key::F10),
            122 => Some(Key::F11),
            123 => Some(Key::F12),
            124 => Some(Key::F13),
            125 => Some(Key::F14),
            126 => Some(Key::F15),
            127 => Some(Key::F16),
            128 => Some(Key::F17),
            129 => Some(Key::F18),
            130 => Some(Key::F19),
            131 => Some(Key::F20),
            132 => Some(Key::F21),
            133 => Some(Key::F22),
            134 => Some(Key::F23),
            135 => Some(Key::F24),
            144 => Some(Key::Numlock),
            145 => Some(Key::Scroll),
            160 => Some(Key::LeftShift),
            161 => Some(Key::RightShift),
            162 => Some(Key::LeftControl),
            163 => Some(Key::RightControl),
            164 => Some(Key::LeftMenu),
            165 => Some(Key::RightMenu),
            166 => Some(Key::BrowserBack),
            167 => Some(Key::BrowserForward),
            168 => Some(Key::BrowserRefresh),
            169 => Some(Key::BrowserStop),
            170 => Some(Key::BrowserSearch),
            171 => Some(Key::BrowserFavorites),
            172 => Some(Key::BrowserHome),
            173 => Some(Key::VolumeMute),
            174 => Some(Key::VolumeDown),
            175 => Some(Key::VolumeUp),
            176 => Some(Key::NextTrack),
            177 => Some(Key::PrevTrack),
            178 => Some(Key::MediaStop),
            179 => Some(Key::MediaPlayPause),
            180 => Some(Key::StartMail),
            181 => Some(Key::SelectMedia),
            182 => Some(Key::StartApp1),
            183 => Some(Key::StartApp2),
            186 => Some(Key::Oem1),
            187 => Some(Key::Plus),
            188 => Some(Key::Comma),
            189 => Some(Key::Minus),
            190 => Some(Key::Period),
            191 => Some(Key::Oem2),
            192 => Some(Key::Oem3),
            219 => Some(Key::Oem4),
            220 => Some(Key::Oem5),
            221 => Some(Key::Oem6),
            222 => Some(Key::Oem7),
            223 => Some(Key::Oem8),
            226 => Some(Key::Oem102),
            229 => Some(Key::ImeProcess),
            246 => Some(Key::Attn),
            247 => Some(Key::CrSel),
            248 => Some(Key::ExSel),
            249 => Some(Key::EraseEof),
            250 => Some(Key::Play),
            251 => Some(Key::Zoom),
            253 => Some(Key::Pa1),
            254 => Some(Key::OemClear),
            _ => None,
        }
    }

    /// Checks if this key is currently being pressed.
    pub fn is_down(self) -> bool {
        plat::key_is_down(self)
    }
}

/// Anything that can be pressed like a key: a [`Key`] or a `char`.
///
/// This mirrors the ergonomics of `winput::Keylike`, so both of these work:
///
/// ```no_run
/// use rsautogui::keyboard::{self, Key};
/// keyboard::key_tap(Key::A);
/// keyboard::key_tap('A');
/// ```
pub trait Keylike: Copy {
    /// Presses the key down.
    fn press(self);
    /// Releases the key.
    fn release(self);
}

impl Keylike for Key {
    #[inline]
    fn press(self) {
        plat::key_press(self);
    }
    #[inline]
    fn release(self) {
        plat::key_release(self);
    }
}

impl Keylike for char {
    #[inline]
    fn press(self) {
        plat::char_press(self);
    }
    #[inline]
    fn release(self) {
        plat::char_release(self);
    }
}

/// Simulates typing the string provided.
pub fn typewrite(string: &str) {
    plat::typewrite_str(string);
}

/// Presses specified key down.
pub fn key_down<K: Keylike>(key: K) {
    key.press();
}

/// Releases specified key up.
pub fn key_up<K: Keylike>(key: K) {
    key.release();
}

/// Performs specified key down and up (a tap).
pub fn key_tap<K: Keylike>(key: K) {
    key.press();
    key.release();
}

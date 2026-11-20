// The bridged click.
//
// The 1.0.1 `.hnmod` swapped the tick's crosshair for one use and let the
// game's own use-on-block path place the block. A mod cannot reach that path
// from the SDK, so this builds the same vanilla `use_item_on` packet the
// game would have sent for such a click and submits it through
// `ClientHost::send_packet`.
//
// What the server then does is exactly what it does for a player's click on
// that block face: the same reach, survival, `can_place_on`, item and
// protection checks. A refused click is refused by the server, so nothing is
// rolled back here.

use mod_api::prelude::*;
use mod_api::{ClientHost, Hand, Identifier, PlayerRef, ServerboundPlayPacket};

use crate::runtime::{Dir, Hit};

/// The vanilla `use_item_on` packet id, as the protocol spells it.
const USE_ITEM_ON: &str = "minecraft:use_item_on";

/// A packet body writer for the fields vanilla reads, in its order.
struct Body {
    bytes: Vec<u8>,
}

impl Body {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// A VarInt: seven bits a byte, low group first, the top bit continuing.
    fn varint(&mut self, mut value: i32) {
        let mut v = value as u32;
        loop {
            let byte = (v & 0x7F) as u8;
            v >>= 7;
            if v == 0 {
                self.bytes.push(byte);
                break;
            }
            self.bytes.push(byte | 0x80);
        }
        let _ = &mut value;
    }

    fn bool(&mut self, value: bool) {
        self.bytes.push(u8::from(value));
    }

    fn f32(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// `BlockPos.asLong`: x and z in 26 bits each, y in 12, packed in a long.
    fn pos(&mut self, pos: [i32; 3]) {
        let [x, y, z] = pos;
        let packed = ((x as i64 & 0x3FF_FFFF) << 38)
            | ((z as i64 & 0x3FF_FFFF) << 12)
            | (y as i64 & 0xFFF);
        self.bytes.extend_from_slice(&packed.to_be_bytes());
    }
}

/// The face's number, as the protocol writes it: down, up, north, south, west, east.
fn face_id(face: Dir) -> i32 {
    match face {
        Dir::Down => 0,
        Dir::Up => 1,
        Dir::North => 2,
        Dir::South => 3,
        Dir::West => 4,
        Dir::East => 5,
    }
}

/// Send one bridged click at `hit`, with `hand` (0 main, 1 off).
///
/// The click is checked with the game's own `may_use_item_at` first, the read
/// the 1.0.1 bundle used to refuse a click it knew the server would reject.
pub fn place(host: &mut dyn ClientHost, player: PlayerRef, hand: usize, hit: &Hit) -> bool {
    let hand = if hand == 0 { Hand::Main } else { Hand::Off };
    let Some(stack) = host
        .living(player.entity())
        .and_then(|living| living.hand_item(host, hand))
    else {
        return false;
    };
    // The game's own placement rules for that position and face.
    if host
        .player(player.uuid())
        .and_then(|p| {
            p.may_use_item_at(host, [hit.pos.x, hit.pos.y, hit.pos.z], face(hit.face), &stack)
        })
        == Some(false)
    {
        return false;
    }

    let mut body = Body::new();
    // `hand`: 0 main, 1 off.
    body.varint(if matches!(hand, Hand::Main) { 0 } else { 1 });
    body.pos([hit.pos.x, hit.pos.y, hit.pos.z]);
    body.varint(face_id(hit.face));
    // The cursor, relative to the clicked block.
    let cursor = [
        (hit.point[0] - f64::from(hit.pos.x)) as f32,
        (hit.point[1] - f64::from(hit.pos.y)) as f32,
        (hit.point[2] - f64::from(hit.pos.z)) as f32,
    ];
    for c in cursor {
        body.f32(c);
    }
    // `inside`: the ray started inside the clicked block's shape.
    body.bool(false);
    // `world_border`: not at the border.
    body.bool(false);
    // `sequence`: the prediction sequence the server acknowledges with.
    body.varint(0);

    let Ok(id) = Identifier::parse(USE_ITEM_ON) else {
        return false;
    };
    host.send_packet(ServerboundPlayPacket::new(id, body.bytes))
        .is_ok()
}

/// The `BlockFace` the protocol's number names, for `may_use_item_at`.
fn face(face: Dir) -> mod_api::BlockFace {
    match face {
        Dir::Down => mod_api::BlockFace::Down,
        Dir::Up => mod_api::BlockFace::Up,
        Dir::North => mod_api::BlockFace::North,
        Dir::South => mod_api::BlockFace::South,
        Dir::West => mod_api::BlockFace::West,
        Dir::East => mod_api::BlockFace::East,
    }
}

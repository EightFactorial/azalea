use azalea_buf::McBuf;
use packet_macros::GamePacket;

#[derive(Clone, Debug, McBuf, GamePacket)]
pub struct ClientboundAnimatePacket {
    #[var]
    pub id: u32,
    pub action: AnimationAction,
}

// minecraft actually uses a u8 for this, but a varint still works and makes it
// so i don't have to add a special handler
#[derive(Clone, Debug, Copy, McBuf)]
pub enum AnimationAction {
    SwingMainHand = 0,
    Hurt = 1,
    WakeUp = 2,
    SwingOffHand = 3,
    CriticalHit = 4,
    MagicCriticalHit = 5,
}
use drm::ClientCapability as CC;
pub const CLIENT_CAP_ENUMS: &[CC] = &[CC::Stereo3D, CC::UniversalPlanes, CC::Atomic];

use drm::DriverCapability as DC;
pub const DRIVER_CAP_ENUMS: &[DC] = &[
    DC::DumbBuffer,
    DC::VBlankHighCRTC,
    DC::DumbPreferredDepth,
    DC::DumbPreferShadow,
    DC::Prime,
    DC::MonotonicTimestamp,
    DC::ASyncPageFlip,
    DC::CursorWidth,
    DC::CursorHeight,
    DC::AddFB2Modifiers,
    DC::PageFlipTarget,
    DC::CRTCInVBlankEvent,
    DC::SyncObj,
    DC::TimelineSyncObj,
];

pub fn enable_client_cap<T: drm::Device>(card: &T) {
    for &cap in CLIENT_CAP_ENUMS {
        if let Err(e) = card.set_client_capability(cap, true) {
            eprintln!("Unable to activate client capability {:?}: {}", cap, e);
            return;
        }
    }
}

pub fn get_driver_cap<T: drm::Device>(card: &T) {
    for &cap in DRIVER_CAP_ENUMS {
        println!("{:?}: {:?}", cap, card.get_driver_capability(cap));
    }
}

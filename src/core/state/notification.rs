use std::time::Instant;

#[derive(Eq, PartialEq, Debug)]
pub enum NotificationKind {
    Activity,
    NoActivity,
}

#[derive(Debug)]
pub struct Notification {
    pub kind: NotificationKind,
    pub timestamp: Instant,
}

impl Notification {
    pub fn new(kind: NotificationKind) -> Notification {
        Notification {
            kind,
            timestamp: Instant::now(),
        }
    }
}

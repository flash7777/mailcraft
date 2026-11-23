//! Keeping the last months of mail (`SyncOptions::keep_months`), against the in-memory server.

use super::*;

/// A message with a `Date:` header `days_ago` days before now (the fake's day for `SEARCH SINCE`).
fn dated(subject: &str, days_ago: i64) -> Vec<u8> {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let (year, month, day) = mailcraft_model::dates::civil_from_days(now.div_euclid(86_400) - days_ago);
    let name = MONTHS[month as usize - 1];
    format!("Subject: {subject}\r\nMessage-ID: <{subject}@x>\r\nDate: {day} {name} {year} 09:00:00 +0000\r\n\r\nbody\r\n").into_bytes()
}

fn subjects(store: &Store) -> Vec<String> {
    let mut kept: Vec<String> = store.load_folder("a1", "INBOX").unwrap().messages.iter().map(|m| m.subject.clone()).collect();
    kept.sort();
    kept
}

/// The first sync keeps what came in the last months, not the newest numbers: a mailbox moved in
/// from elsewhere has its old mail on the highest UIDs. Later syncs stay within the months, and
/// Show more (the folder's own window, past the months, as the worker asks it) brings older mail.
#[test]
fn a_folder_keeps_the_last_months_by_date_not_by_number() {
    let st = State::new();
    {
        let mut s = st.lock().unwrap();
        s.add_raw("INBOX", dated("recent-a", 3));
        s.add_raw("INBOX", dated("recent-b", 40));
        s.add_raw("INBOX", dated("recent-c", 100));
        // Moved in last: the newest numbers, but old mail.
        s.add_raw("INBOX", dated("old-a", 400));
        s.add_raw("INBOX", dated("old-b", 700));
    }
    let store = store("keep-months");
    let mut c = login(&st);
    let months = SyncOptions { window: usize::MAX, prefetch: 0, keep_months: 6, ..Default::default() };
    assert_eq!(sync_folder(&mut c, &store, "a1", "INBOX", &months).unwrap().new, 3);
    assert_eq!(subjects(&store), ["recent-a", "recent-b", "recent-c"]);

    // New mail comes; the older backlog doesn't.
    st.lock().unwrap().add_raw("INBOX", dated("new", 0));
    assert_eq!(sync_folder(&mut c, &store, "a1", "INBOX", &months).unwrap().new, 1);
    assert_eq!(sync_folder(&mut c, &store, "a1", "INBOX", &months).unwrap().new, 0, "once");

    // Show more: one more than kept, months set aside.
    let more = SyncOptions { window: 5, keep_months: 0, ..months };
    assert_eq!(sync_folder(&mut c, &store, "a1", "INBOX", &more).unwrap().new, 1);
    assert_eq!(subjects(&store), ["new", "old-b", "recent-a", "recent-b", "recent-c"]);
}

/// Without months, the first sync takes the newest numbers, as before.
#[test]
fn without_months_the_first_sync_takes_the_newest_numbers() {
    let st = State::new();
    {
        let mut s = st.lock().unwrap();
        s.add_raw("INBOX", dated("recent", 3));
        s.add_raw("INBOX", dated("old", 400));
    }
    let store = store("keep-months-none");
    let mut c = login(&st);
    sync_folder(&mut c, &store, "a1", "INBOX", &SyncOptions { window: 1, prefetch: 0, ..Default::default() }).unwrap();
    assert_eq!(subjects(&store), ["old"]);
}

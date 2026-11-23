//! Engine tests: a message, or a message file opened to look at, edited as a new message.

use super::*;

/// A message file written elsewhere, from the sample account's own address, with a file
/// attached and an answer's headers.
fn prepared_offer() -> String {
    concat!(
        "Date: Tue, 24 Nov 2026 09:00:00 +0100\r\n",
        "Message-ID: <offer-1@studio.example>\r\n",
        "In-Reply-To: <request-1@greenhouse.example>\r\n",
        "References: <request-1@greenhouse.example>\r\n",
        "Subject: Offer for the greenhouse\r\n",
        "From: Riley Moreno <riley@studio.example>\r\n",
        "To: Lee Park <lee@greenhouse.example>\r\n",
        "Cc: mo@farm.example\r\n",
        "MIME-Version: 1.0\r\n",
        "Content-Type: multipart/mixed; boundary=\"b1\"\r\n",
        "\r\n",
        "--b1\r\n",
        "Content-Type: text/plain; charset=utf-8\r\n",
        "\r\n",
        "Dear Lee,\r\n",
        "the offer is attached.\r\n",
        "--b1\r\n",
        "Content-Type: application/pdf\r\n",
        "Content-Disposition: attachment; filename=\"Offer.pdf\"\r\n",
        "Content-Transfer-Encoding: base64\r\n",
        "\r\n",
        "JVBERi0xLjQK\r\n",
        "--b1--\r\n",
    )
    .to_string()
}

#[test]
fn an_opened_message_file_is_edited_as_a_new_message_and_sent() {
    let mut s = sample_session("edit-as-new-file");
    let shown = s.open_message_file("Offer.eml", prepared_offer().as_bytes()).unwrap();
    let file = shown["key"].as_str().unwrap().to_string();

    s.run("mail.editAsNew", &json!({ "key": file })).unwrap();
    let c = s.compose.clone().unwrap();
    assert_eq!(c.account, "sample", "from the account whose address it's from");
    assert_eq!(c.from, "", "its own address");
    assert_eq!(c.to, "Lee Park <lee@greenhouse.example>; ");
    assert_eq!(c.cc, "mo@farm.example; ");
    assert_eq!(c.subject, "Offer for the greenhouse");
    assert!(c.body.contains("the offer is attached."), "{:?}", c.body);
    assert_eq!(c.attachments.len(), 1);
    assert_eq!((c.attachments[0].name.as_str(), c.attachments[0].data.as_slice()), ("Offer.pdf", b"%PDF-1.4\n".as_slice()));
    // A message of its own: no draft of anything, an answer to nothing.
    assert!(c.draft.is_none() && c.draft_id.is_empty() && c.source.is_none() && c.mode.is_none());
    assert!(c.in_reply_to.is_empty() && c.references.is_empty());
    assert!(s.windows.iter().any(|w| w.key.to_id() == file), "the file's window stays open");

    s.run("compose.send", &json!({})).unwrap();
    assert!(s.compose.is_none());
    s.run("folder.open", &json!({"account": "sample", "folder": "Sent"})).unwrap();
    let sent = s.list.iter().find(|m| m.subject == "Offer for the greenhouse").unwrap().key.clone();
    let raw = String::from_utf8_lossy(&s.body_raw(&sent).unwrap()).to_string();
    assert!(!raw.contains("offer-1@studio.example"), "a Message-ID of its own");
    assert!(!raw.contains("In-Reply-To") && !raw.contains("request-1@"), "it answers nothing");
    assert!(raw.contains("Offer.pdf"), "its file goes with it");
}

#[test]
fn a_received_message_is_edited_as_new_and_left_as_it_was() {
    let mut s = sample_session("edit-as-new-inbox");
    let k = key_of(&s, "Your proof for the festival poster is ready");
    s.run("list.open", &k).unwrap();
    let before = s.reading.as_ref().unwrap().key.clone();

    s.run("mail.editAsNew", &json!({})).unwrap();
    let c = s.compose.clone().unwrap();
    assert_eq!(c.subject, "Your proof for the festival poster is ready", "no Re: or Fwd:");
    assert!(c.body.contains("The proof for order 4471"), "{:?}", c.body);
    assert_eq!(c.account, "sample", "not from a sender's address here: from its own account");
    assert!(c.draft.is_none() && c.source.is_none());

    // Closed unchanged, it leaves nothing behind, and the original is still in the Inbox.
    s.run("compose.discard", &json!({})).unwrap();
    s.run("folder.open", &json!({"account": "sample", "folder": "Drafts"})).unwrap();
    assert!(s.list.is_empty());
    s.run("folder.open", &json!({"account": "sample", "folder": "INBOX"})).unwrap();
    assert!(s.list.iter().any(|m| m.key == before));
}

#[test]
fn nothing_open_is_nothing_to_edit() {
    let mut s = sample_session("edit-as-new-none");
    s.reading = None;
    assert!(matches!(s.run("mail.editAsNew", &json!({})), Err(CmdError::Disabled(_))));
    assert!(s.run("mail.editAsNew", &json!({ "key": "not a key" })).is_err());
    assert!(s.compose.is_none());
}

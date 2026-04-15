//! Parse notification / diagnostic system.
//!
//! Mirrors ACadSharp's `NotificationEventHandler` pattern.  Non-fatal issues
//! encountered during reading (or writing) are collected as `Notification`
//! items rather than being silently dropped or causing hard errors.
//!
//! After a read/write operation the caller can inspect
//! [`CadDocument::notifications`](crate::document::CadDocument::notifications) to see what was encountered.

use std::fmt;
use crate::types::Handle;

/// Severity level of a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NotificationType {
    /// An entity/object/section is not yet implemented.
    NotImplemented,
    /// Feature exists but is not supported in this context.
    NotSupported,
    /// Non-fatal warning (e.g., missing handle, duplicate key).
    Warning,
    /// Error that was recovered from (e.g., bad group code value).
    Error,
}

impl fmt::Display for NotificationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplemented => write!(f, "NotImplemented"),
            Self::NotSupported => write!(f, "NotSupported"),
            Self::Warning => write!(f, "Warning"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// A single notification produced during reading or writing.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Notification {
    /// The severity / category.
    pub notification_type: NotificationType,
    /// A human-readable description of the issue.
    pub message: String,
}

impl Notification {
    /// Create a new notification.
    pub fn new(notification_type: NotificationType, message: impl Into<String>) -> Self {
        Self {
            notification_type,
            message: message.into(),
        }
    }
}

impl fmt::Display for Notification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.notification_type, self.message)
    }
}

/// Collects notifications during a read/write operation.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NotificationCollection {
    items: Vec<Notification>,
}

impl NotificationCollection {
    /// Create an empty collection.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Record a notification.
    pub fn notify(&mut self, notification_type: NotificationType, message: impl Into<String>) {
        self.items.push(Notification::new(notification_type, message));
    }

    /// Check if there are any notifications.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Number of notifications.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Iterate over all notifications.
    pub fn iter(&self) -> std::slice::Iter<'_, Notification> {
        self.items.iter()
    }

    /// Get all notifications of a specific type.
    pub fn of_type(&self, nt: NotificationType) -> Vec<&Notification> {
        self.items.iter().filter(|n| n.notification_type == nt).collect()
    }

    /// Check whether any notification of the given type exists.
    pub fn has_type(&self, nt: NotificationType) -> bool {
        self.items.iter().any(|n| n.notification_type == nt)
    }

    /// Append all notifications from another collection.
    pub fn extend(&mut self, other: NotificationCollection) {
        self.items.extend(other.items);
    }

    /// Consume the collection into a `Vec`.
    pub fn into_vec(self) -> Vec<Notification> {
        self.items
    }
}

impl IntoIterator for NotificationCollection {
    type Item = Notification;
    type IntoIter = std::vec::IntoIter<Notification>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a NotificationCollection {
    type Item = &'a Notification;
    type IntoIter = std::slice::Iter<'a, Notification>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

/// Document mutation event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DocumentEventType {
    /// An entity was added to the document.
    EntityAdded,
    /// An entity was removed from the document.
    EntityRemoved,
    /// An existing entity was modified.
    EntityModified,
    /// Undo operation was recorded.
    Undo,
    /// Redo operation was recorded.
    Redo,
}

/// A single document mutation event.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DocumentEvent {
    /// Event kind.
    pub event_type: DocumentEventType,
    /// Entity handle involved in this event, if applicable.
    pub handle: Option<Handle>,
    /// Optional human-readable context.
    pub message: Option<String>,
}

impl DocumentEvent {
    /// Create an event with optional handle/message.
    pub fn new(
        event_type: DocumentEventType,
        handle: Option<Handle>,
        message: Option<impl Into<String>>,
    ) -> Self {
        Self {
            event_type,
            handle,
            message: message.map(|m| m.into()),
        }
    }
}

/// Collection of document mutation events.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DocumentEventCollection {
    items: Vec<DocumentEvent>,
}

impl DocumentEventCollection {
    /// Create an empty event collection.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Push a raw event.
    pub fn push(&mut self, event: DocumentEvent) {
        self.items.push(event);
    }

    /// Record an entity-added event.
    pub fn entity_added(&mut self, handle: Handle) {
        self.push(DocumentEvent::new(DocumentEventType::EntityAdded, Some(handle), None::<String>));
    }

    /// Record an entity-removed event.
    pub fn entity_removed(&mut self, handle: Handle) {
        self.push(DocumentEvent::new(DocumentEventType::EntityRemoved, Some(handle), None::<String>));
    }

    /// Record an entity-modified event.
    pub fn entity_modified(&mut self, handle: Handle, message: impl Into<String>) {
        self.push(DocumentEvent::new(
            DocumentEventType::EntityModified,
            Some(handle),
            Some(message),
        ));
    }

    /// Number of events.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are no events.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate events.
    pub fn iter(&self) -> std::slice::Iter<'_, DocumentEvent> {
        self.items.iter()
    }

    /// Clear all recorded events.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Drain all events and return them.
    pub fn drain(&mut self) -> Vec<DocumentEvent> {
        std::mem::take(&mut self.items)
    }
}

impl IntoIterator for DocumentEventCollection {
    type Item = DocumentEvent;
    type IntoIter = std::vec::IntoIter<DocumentEvent>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a DocumentEventCollection {
    type Item = &'a DocumentEvent;
    type IntoIter = std::slice::Iter<'a, DocumentEvent>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let n = Notification::new(NotificationType::Warning, "handle missing");
        assert_eq!(n.notification_type, NotificationType::Warning);
        assert_eq!(n.message, "handle missing");
    }

    #[test]
    fn test_collection_basics() {
        let mut c = NotificationCollection::new();
        assert!(c.is_empty());

        c.notify(NotificationType::Warning, "w1");
        c.notify(NotificationType::Error, "e1");
        c.notify(NotificationType::Warning, "w2");

        assert_eq!(c.len(), 3);
        assert_eq!(c.of_type(NotificationType::Warning).len(), 2);
        assert!(c.has_type(NotificationType::Error));
        assert!(!c.has_type(NotificationType::NotImplemented));
    }

    #[test]
    fn test_display() {
        let n = Notification::new(NotificationType::NotImplemented, "THUMBNAILIMAGE section");
        assert_eq!(format!("{}", n), "[NotImplemented] THUMBNAILIMAGE section");
    }

    #[test]
    fn test_document_event_collection_basics() {
        let mut events = DocumentEventCollection::new();
        assert!(events.is_empty());

        let h1 = Handle::new(0x10);
        let h2 = Handle::new(0x11);
        events.entity_added(h1);
        events.entity_modified(h1, "translated");
        events.entity_removed(h2);

        assert_eq!(events.len(), 3);
        assert_eq!(events.iter().next().unwrap().event_type, DocumentEventType::EntityAdded);
        assert_eq!(events.iter().nth(1).unwrap().event_type, DocumentEventType::EntityModified);
        assert_eq!(events.iter().nth(2).unwrap().event_type, DocumentEventType::EntityRemoved);
    }

    #[test]
    fn test_document_event_drain() {
        let mut events = DocumentEventCollection::new();
        events.entity_added(Handle::new(0x20));
        let drained = events.drain();
        assert_eq!(drained.len(), 1);
        assert!(events.is_empty());
    }
}

use core::fmt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowRef {
    pub id: u64,
    pub generation: u64,
}

impl WindowRef {
    #[must_use]
    pub const fn new(id: u64, generation: u64) -> Self {
        Self { id, generation }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.id != 0
    }
}

impl fmt::Display for WindowRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.id, self.generation)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceRef {
    pub index: i32,
    pub revision: u64,
}

impl WorkspaceRef {
    #[must_use]
    pub const fn new(index: i32, revision: u64) -> Self {
        Self { index, revision }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.index >= 0
    }
}

impl fmt::Display for WorkspaceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.index, self.revision)
    }
}

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn is_valid(&self) -> bool {
                !self.0.is_empty()
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(OutputId);
string_id!(DesktopAppId);
string_id!(TaskEntryId);

impl TaskEntryId {
    #[must_use]
    pub fn pinned(app: &DesktopAppId) -> Self {
        Self(format!("pin:{}", app.as_str()))
    }

    #[must_use]
    pub fn window(window: WindowRef) -> Self {
        Self(format!("win:{}:{}", window.id, window.generation))
    }

    #[must_use]
    pub fn is_pinned(&self) -> bool {
        self.0.starts_with("pin:")
    }

    #[must_use]
    pub fn is_window(&self) -> bool {
        self.0.starts_with("win:")
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModeId(pub u64);

impl ModeId {
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(pub u64);

impl TransactionId {
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

id_type!(UserId);
id_type!(ProjectId);
id_type!(AssetId);
id_type!(DocumentId);
id_type!(ParagraphId);
id_type!(CitationId);
id_type!(ReferenceEntryId);
id_type!(JobId);
id_type!(SyncRecordId);

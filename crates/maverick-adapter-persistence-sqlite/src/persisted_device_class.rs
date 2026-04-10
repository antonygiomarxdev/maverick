//! Closed set of persisted `device_class` tags (adapter boundary; must match DB CHECK-free TEXT).

use maverick_domain::DeviceClass;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersistedDeviceClassTag {
    ClassA,
    ClassB,
    ClassC,
}

impl PersistedDeviceClassTag {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ClassA => "ClassA",
            Self::ClassB => "ClassB",
            Self::ClassC => "ClassC",
        }
    }
}

impl Display for PersistedDeviceClassTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for PersistedDeviceClassTag {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        [Self::ClassA, Self::ClassB, Self::ClassC]
            .into_iter()
            .find(|t| t.as_str() == value)
            .ok_or(())
    }
}

impl From<DeviceClass> for PersistedDeviceClassTag {
    fn from(value: DeviceClass) -> Self {
        match value {
            DeviceClass::ClassA => Self::ClassA,
            DeviceClass::ClassB => Self::ClassB,
            DeviceClass::ClassC => Self::ClassC,
        }
    }
}

impl From<PersistedDeviceClassTag> for DeviceClass {
    fn from(value: PersistedDeviceClassTag) -> Self {
        match value {
            PersistedDeviceClassTag::ClassA => DeviceClass::ClassA,
            PersistedDeviceClassTag::ClassB => DeviceClass::ClassB,
            PersistedDeviceClassTag::ClassC => DeviceClass::ClassC,
        }
    }
}

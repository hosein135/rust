use rustdv_gpi_sys as sys;

/// Convert a two-state value to a fixed-size array of VPI vector words.
pub trait ToVpiWords<const N: usize> {
    fn to_vpi_words(self) -> [sys::t_vpi_vecval; N];
}

impl ToVpiWords<1> for bool {
    fn to_vpi_words(self) -> [sys::t_vpi_vecval; 1] {
        [sys::t_vpi_vecval {
            aval: self as u32,
            bval: 0,
        }]
    }
}

impl ToVpiWords<1> for u8 {
    fn to_vpi_words(self) -> [sys::t_vpi_vecval; 1] {
        [sys::t_vpi_vecval {
            aval: self as u32,
            bval: 0,
        }]
    }
}

impl ToVpiWords<1> for u16 {
    fn to_vpi_words(self) -> [sys::t_vpi_vecval; 1] {
        [sys::t_vpi_vecval {
            aval: self as u32,
            bval: 0,
        }]
    }
}

impl ToVpiWords<1> for u32 {
    fn to_vpi_words(self) -> [sys::t_vpi_vecval; 1] {
        [sys::t_vpi_vecval {
            aval: self,
            bval: 0,
        }]
    }
}

impl ToVpiWords<2> for u64 {
    fn to_vpi_words(self) -> [sys::t_vpi_vecval; 2] {
        [
            sys::t_vpi_vecval {
                aval: self as u32,
                bval: 0,
            },
            sys::t_vpi_vecval {
                aval: (self >> 32) as u32,
                bval: 0,
            },
        ]
    }
}

impl ToVpiWords<4> for u128 {
    fn to_vpi_words(self) -> [sys::t_vpi_vecval; 4] {
        [
            sys::t_vpi_vecval {
                aval: self as u32,
                bval: 0,
            },
            sys::t_vpi_vecval {
                aval: (self >> 32) as u32,
                bval: 0,
            },
            sys::t_vpi_vecval {
                aval: (self >> 64) as u32,
                bval: 0,
            },
            sys::t_vpi_vecval {
                aval: (self >> 96) as u32,
                bval: 0,
            },
        ]
    }
}

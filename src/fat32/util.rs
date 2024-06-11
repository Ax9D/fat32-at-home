macro_rules! read_bytes {
    ($ty: ty, $reader: expr, $err: expr) => {
        {
            let mut buf= [0_u8; std::mem::size_of::<$ty>()];

            $reader.read_exact(&mut buf).map_err(|_| Fat32Error::InvalidBPB($err.into()))?;

            <$ty>::from_le_bytes(buf)
        }
    };
}

pub(crate) use read_bytes;
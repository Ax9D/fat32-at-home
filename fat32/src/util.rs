macro_rules! read_bytes {
    ($ty: ty, $reader: expr) => {
        {
            let mut buf= [0_u8; std::mem::size_of::<$ty>()];

            $reader.read_exact(&mut buf).map_err(|err| Fat32Error::IOError(err))
            .map(|_| <$ty>::from_le_bytes(buf))
        }
    };
}

pub(crate) use read_bytes;
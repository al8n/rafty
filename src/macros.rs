#[allow(unused_macros)]

macro_rules! feature {
    (
        #![$meta:meta]
        $($item:item)*
    ) => {
        $(
            #[cfg($meta)]
            #[cfg_attr(docsrs, doc(cfg($meta)))]
            $item
        )*
    }
}

macro_rules! cfg_sync {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "sync")]
            #[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
            $item
        )*
    }
}

macro_rules! cfg_default {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "default")]
            #[cfg_attr(docsrs, doc(cfg(feature = "default")))]
            $item
        )*
    }
}

macro_rules! cfg_not_default {
    ($($item:item)*) => {
        $(
            #[cfg(not(feature = "default"))]
            #[cfg_attr(docsrs, doc(cfg(not(feature = "default"))))]
            $item
        )*
    }
}

macro_rules! cfg_test {
    ($($item:item)*) => {
        $(
            #[cfg(test)]
            #[cfg_attr(docsrs, doc(cfg(test)))]
            $item
        )*
    }
}
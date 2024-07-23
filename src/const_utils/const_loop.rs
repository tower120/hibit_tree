// godbolt shows that unreachable cases does not generate code even in opt-level=0.
/// Compile-time loop. Loop iterate over **CONST** usize. 
/// 
/// Each iteration are guaranteed to be separate piece of code.
macro_rules! const_loop {
    // Looks like a little bit faster to compile?
    (@internal_fwd $n:ident in {$($is:tt),*} to $end:tt => $break_label:lifetime : $body:block) => {
        $(
            if $end == $is {break $break_label;}
            {
                const $n: usize = $is;
                $body
            } 
        )*
    };

    // Universal, can be used by forward loop as well.
    (@internal_rev $n:ident in {$($is:tt),*} to $end:tt => $body:block) => {
        $(
            if $is < $end {
                const $n: usize = $is;
                $body
            } 
        )*
    };    
    
    ($n:ident in 0..$end:tt => $body:block) => {
        const_loop!($n in 0..$end => 'out: $body);
    };

    ($n:ident in 0..$end:tt => $break_label:lifetime : $body:block) => {
        $break_label: {
            const{ 
                // Also, this ensures that "end" is compile-time'able. 
                assert!($end < 9, "const_loop end bound too high.");
            }
            const_loop!(@internal_fwd $n in {0,1,2,3,4,5,6,7,8} to $end => $break_label: $body);
        }
    };

    ($n:ident in 0..$end:tt rev => $body:block) => {
        const_loop!($n in 0..$end rev => 'out: $body);
    };

    ($n:ident in 0..$end:tt rev => $break_label:lifetime : $body:block) => {
        $break_label: {
            const{ 
                // Also, this ensures that "end" is compile-time'able. 
                assert!($end < 9, "const_loop end bound too high.");
            }
            const_loop!(@internal_rev $n in {8,7,6,5,4,3,2,1,0} to $end => $body);
        }
    };
}
pub(crate) use const_loop;

#[cfg(test)]
mod test{
    use crate::const_utils::{ConstInteger, ConstUsize};
    use super::*;
    
    #[test]
    fn const_for_macro_test(){
        const_loop!(n1 in 0..3 => { println!("aa {:?}", n1) });

        fn test<I: ConstInteger>(_: I){
            let i = 5;
            const_loop!(n in 0..{<I as ConstInteger>::VALUE} => 'out: {
                if n>i {break 'out;}
                
                println!("aa {:?}", n)
                
            });    
        }
        test(ConstUsize::<7>);
    }

    #[test]
    fn const_for_rev_macro_test(){
        const_loop!(n1 in 0..3 rev => { println!("aa {:?}", n1) });

        fn test<I: ConstInteger>(_: I){
            let i = 2;
            const_loop!(n in 0..{<I as ConstInteger>::VALUE} rev => 'out: {
                if n<i {break 'out;}
                println!("bb {:?}", n)
            });    
        }
        test(ConstUsize::<7>);
    }
    
}
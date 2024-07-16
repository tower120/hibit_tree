// godbolt shows that unreachable cases does not generate code even in opt-level=0.
/// Compile-time loop. Loop iterate over **CONST** usize. 
/// 
/// Each iteration are guaranteed to be separate piece of code.
macro_rules! const_loop {
    ($n:ident in 0..$end:tt => $body:block) => {
        const_loop!($n in 0..$end => 'out: $body);
    };

    ($n:ident in 0..$end:tt => $break_label:lifetime : $body:block) => {
        $break_label: {
            const{ 
                // Also, this ensures that "end" is compile-time'able. 
                assert!($end < 9, "const_for end bound too high.");
            }

            if $end == 0 {break $break_label;}
            {
                const $n: usize = 0;
                $body
            }

            if $end == 1 {break $break_label;}
            {
                const $n: usize = 1;
                $body
            }

            if $end == 2 {break $break_label;}
            {
                const $n: usize = 2;
                $body
            }

            if $end == 3 {break $break_label;}
            {
                const $n: usize = 3;
                $body
            }

            if $end == 4 {break $break_label;}
            {
                const $n: usize = 4;
                $body
            }

            if $end == 5 {break $break_label;}
            {
                const $n: usize = 5;
                $body
            }

            if $end == 6 {break $break_label;}
            {
                const $n: usize = 6;
                $body
            }

            if $end == 7 {break $break_label;}
            {
                const $n: usize = 7;
                $body
            }

            if $end == 8 {break $break_label;}
            {
                const $n: usize = 8;
                $body
            }
        }
    };

    // TODO: rev version
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
}
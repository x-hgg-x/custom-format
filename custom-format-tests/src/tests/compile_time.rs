use custom_format::compile_time as cfmt;

use core::fmt;

#[test]
fn test_format_args_macro() {
    println!("{}", cfmt::format_args!("string"));
    cfmt::println!("{}", format_args!("string"));
    cfmt::println!("{}", cfmt::format_args!("string"));
}
#[test]
fn test_print_macros() {
    cfmt::print!("string\n");
    cfmt::println!("string");
    cfmt::eprint!("string\n");
    cfmt::eprintln!("string");
}

#[test]
fn test_write_macros() {
    use std::io::Write;

    let mut v = Vec::new();
    let _ = cfmt::write!(v, "string\n");
    let _ = cfmt::writeln!(v, "string");
}

#[test]
#[should_panic(expected = "string")]
fn test_panic_macro() {
    cfmt::panic!("string")
}

#[test]
fn test_no_format_string() {
    cfmt::println!();
    cfmt::eprintln!();
}

#[test]
fn test_literal_format_string() {
    assert_eq!(cfmt::format!("string"), "string");
}

#[test]
fn test_std_fmt() {
    assert_eq!(cfmt::format!("Hello"), "Hello");
    assert_eq!(cfmt::format!("Hello, {}!", "world"), "Hello, world!");
    assert_eq!(cfmt::format!("The number is {}", 1), "The number is 1");
    assert_eq!(cfmt::format!("{:?}", (3, 4)), "(3, 4)");
    assert_eq!(cfmt::format!("{value}", value = 4), "4");
    assert_eq!(cfmt::format!("Hello {people}!", people = "Rustaceans"), "Hello Rustaceans!");
    assert_eq!(cfmt::format!("{} {}", 1, 2), "1 2");
    assert_eq!(cfmt::format!("{:04}", 42), "0042");
    assert_eq!(cfmt::format!("{:#?}", (100, 200)), "(\n    100,\n    200,\n)");
    assert_eq!(cfmt::format!("{1} {} {0} {}", 1, 2), "2 1 1 2");
    assert_eq!(cfmt::format!("{argument}", argument = "test"), "test");
    assert_eq!(cfmt::format!("{name} {}", 1, name = 2), "2 1");
    assert_eq!(cfmt::format!("{a} {c} {b}", a = "a", b = 'b', c = 3), "a 3 b");
    assert_eq!(cfmt::format!("Hello {:5}!", "x"), "Hello x    !");
    assert_eq!(cfmt::format!("Hello {:1$}!", "x", 5), "Hello x    !");
    assert_eq!(cfmt::format!("Hello {1:0$}!", 5, "x"), "Hello x    !");
    assert_eq!(cfmt::format!("Hello {:width$}!", "x", width = 5), "Hello x    !");
    assert_eq!(cfmt::format!("Hello {:<5}!", "x"), "Hello x    !");
    assert_eq!(cfmt::format!("Hello {:-<5}!", "x"), "Hello x----!");
    assert_eq!(cfmt::format!("Hello {:^5}!", "x"), "Hello   x  !");
    assert_eq!(cfmt::format!("Hello {:>5}!", "x"), "Hello     x!");
    assert_eq!(cfmt::format!("Hello {:^15}!", cfmt::format!("{:?}", Some("hi"))), "Hello   Some(\"hi\")   !");
    assert_eq!(cfmt::format!("Hello {:+}!", 5), "Hello +5!");
    assert_eq!(cfmt::format!("{:#x}!", 27), "0x1b!");
    assert_eq!(cfmt::format!("Hello {:05}!", 5), "Hello 00005!");
    assert_eq!(cfmt::format!("Hello {:05}!", -5), "Hello -0005!");
    assert_eq!(cfmt::format!("{:#010x}!", 27), "0x0000001b!");
    assert_eq!(cfmt::format!("Hello {0} is {1:.5}", "x", 0.01), "Hello x is 0.01000");
    assert_eq!(cfmt::format!("Hello {1} is {2:.0$}", 5, "x", 0.01), "Hello x is 0.01000");
    assert_eq!(cfmt::format!("Hello {0} is {2:.1$}", "x", 5, 0.01), "Hello x is 0.01000");
    assert_eq!(cfmt::format!("Hello {} is {:.*}", "x", 5, 0.01), "Hello x is 0.01000");
    assert_eq!(cfmt::format!("Hello {1} is {2:.*}", 5, "x", 0.01), "Hello x is 0.01000");
    assert_eq!(cfmt::format!("Hello {} is {2:.*}", "x", 5, 0.01), "Hello x is 0.01000");
    assert_eq!(cfmt::format!("Hello {} is {number:.prec$}", "x", prec = 5, number = 0.01), "Hello x is 0.01000");
    assert_eq!(cfmt::format!("{}, `{name:.*}`", "Hello", 3, name = 1234.56), "Hello, `1234.560`");
    assert_eq!(cfmt::format!("{}, `{name:.*}`", "Hello", 3, name = "1234.56"), "Hello, `123`");
    assert_eq!(cfmt::format!("{}, `{name:>8.*}`", "Hello", 3, name = "1234.56"), "Hello, `     123`");
    assert_eq!(cfmt::format!("Hello {{}}"), "Hello {}");
    assert_eq!(cfmt::format!("{{ Hello"), "{ Hello");
    assert_eq!(cfmt::format!("{: ^+2$.*e}", 5, -0.01, 15), "  -1.00000e-2  ");
    assert_eq!(cfmt::format!("Hello {::>9.*x? }!", 3, 1.0), "Hello ::::1.000!");

    assert_eq!(cfmt::format!("{h}, {h}, {1}, {1}, {a}, {a}, {3}, {b}, {:.*}", 3, a = 1f64.abs(), b = &(1 + 4), c = 2, h = 0), "0, 0, 1, 1, 1, 1, 2, 5, 1.000");
}

#[test]
fn test_custom_formatter() {
    struct Custom<T>(T);

    impl<T: fmt::Display> fmt::Display for Custom<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    macro_rules! impl_custom_format {
        (match spec { $($spec:literal => $func:expr $(,)?)* }) => {
            $(
                impl<T: fmt::Display> cfmt::CustomFormat<{ cfmt::spec($spec) }> for Custom<T> {
                    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        ($func as fn(&Self, &mut fmt::Formatter) -> fmt::Result)(self, f)
                    }
                }
            )*
        };
    }

    impl_custom_format!(match spec {
        "" => |this, f| write!(f, "({} with spec '{}')", this.0, ""),
        "3xxGxx" => |this, f| write!(f, "({} with spec '{}')", this.0, "3xxGxx"),
    });

    let (g, h) = (Custom(0), Custom(0));

    let result = cfmt::format!(
        "aaaa }} {{}}{} {{{{ \" {:#.*} #{h : } {e \u{3A}3xx\u{47}xx  }, {:?}, { :}, {:?}, {},,{}, {8 :}",
        "ok",
        5,
        Custom(0.01),
        (),
        Custom(1f64.abs()),
        std::format!("{:?}, {}", (3, 4), 5),
        r = &1 + 4,
        b = 2,
        c = Custom(6),
        e = { g },
        h = h,
    );

    assert_eq!(result, "aaaa } {}ok {{ \" 0.01 #(0 with spec '') (0 with spec '3xxGxx'), (), (1 with spec ''), \"(3, 4), 5\", 5,,2, (6 with spec '')");
}

#[test]
fn test_spec() {
    assert_eq!(cfmt::spec(""), 0);
    assert_eq!(cfmt::spec("AB"), 0x4241);
    assert_eq!(cfmt::spec("é"), 0xA9C3);
    assert_eq!(cfmt::spec("\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0~"), 0x7E000000000000000000000000000000);
}

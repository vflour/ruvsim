use ruvsim::sim_types;

use sim_types::{
    LineType, ParsedLine, SimDriver, SimDriverType, SimSignal, SimSignalDirection, SimSignalType,
};

#[test]
fn parsed_line_classification() {
    // TCL comment
    let comment = ParsedLine::new("# this is a tcl comment");
    match comment.line_type {
        LineType::Log => {}
        _ => panic!("Expected Log for comment line"),
    }

    // Prompt
    let prompt = ParsedLine::new("VSIM 25>");
    match prompt.line_type {
        LineType::Prompt => {}
        _ => panic!("Expected Prompt for prompt line"),
    }

    // Output
    let output = ParsedLine::new("Hello world");
    match output.line_type {
        LineType::Output => {}
        _ => panic!("Expected Output for plain output line"),
    }
}

#[test]
fn sim_driver_parsing_single_and_multiple() {
    // Single driver line
    let single = "0: Driver top.u1.signal";
    let drv = SimDriver::from_drivers_output(single);
    match drv.driver_type {
        SimDriverType::Object => {}
        _ => panic!("Expected Object driver type for 'Driver'"),
    }
    assert_eq!(drv.source, "top.u1.signal");

    // Multiple drivers output (as produced by the runner helpers)
    let multi = "Drivers for /top: 0: Driver top.u1.signal; 1: Net top.u2.net;";
    let drivers = SimDriver::from_multiple_drivers_output(multi, "/top");
    assert!(drivers.len() >= 2, "Expected at least two drivers parsed");
    let first = &drivers[0];
    match first.driver_type {
        SimDriverType::Object => {}
        _ => panic!("Expected Object for first driver"),
    }
    assert_eq!(first.source, "top.u1.signal");
    let second = &drivers[1];
    match second.driver_type {
        SimDriverType::Assign => {}
        _ => panic!("Expected Assign (Net) for second driver"),
    }
    assert_eq!(second.source, "top.u2.net");
}

#[test]
fn sim_signal_from_describe_output_parses_type_and_bounds() {
    let drivers: Vec<SimDriver> = Vec::new();
    let out = "Register [7:0] some additional info";
    let sig = SimSignal::from_describe_output("/top/sig", SimSignalDirection::Output, drivers, out);
    match sig.signal_type {
        SimSignalType::Reg => {}
        _ => panic!("Expected Reg signal type"),
    }
    assert_eq!(sig.bounds.left, 7);
    assert_eq!(sig.bounds.right, 0);

    let drivers2: Vec<SimDriver> = Vec::new();
    let out2 = "Wire [15:8] more info";
    let sig2 =
        SimSignal::from_describe_output("/top/sig2", SimSignalDirection::Input, drivers2, out2);
    match sig2.signal_type {
        SimSignalType::Wire => {}
        _ => panic!("Expected Wire signal type"),
    }
    assert_eq!(sig2.bounds.left, 15);
    assert_eq!(sig2.bounds.right, 8);
}

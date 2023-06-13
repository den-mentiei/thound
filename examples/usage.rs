fn main() {
	let info = thound::find_vc_and_windows_sdk();
	if let Some(info) = info {
		if let Some(sdk) = info.sdk {
			println!("Windows SDK is present:");
			println!("{sdk:#?}")
		}

		if let Some(toolchain) = info.toolchain {
			println!("VC toolchain is present:");
			println!("{toolchain:#?}")
		}
	} else {
		eprintln!("We failed to find out the toolchain & SDK information.");
	}
}

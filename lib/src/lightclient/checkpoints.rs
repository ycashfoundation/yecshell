pub fn get_closest_checkpoint(chain_name: &str, height: u64) ->  Option<(u64, &'static str, &'static str)> {
    match chain_name {
        "test" => get_test_checkpoint(height),
        "main" => get_main_checkpoint(height),
        _      => None
    }
}

fn get_test_checkpoint(height: u64) ->  Option<(u64, &'static str, &'static str)> {
    let checkpoints: Vec<(u64, &str, &str)> = vec![
        (350000, "000cdb1eca1bb84e799e73a32a649a1eeec0a1a563d511dfaceaff69a8006527",
         "017f968fad6321e5dde81a4d88a17d262193efccdbfd446f697e2775d25c0b2619014da62eafffb89e4766facabab67199c7fd37c14889d0cce6f9daf96f170ac0060f00017eeb2a8556c7714cbc2502ef958723c1491db8008c9f06858342096880c8333b0139bdb820c2339826cbc6ebc3e8ede79004f865d4d48233e74e21d0cf4821163200000001aac1d37ab43d4417be4e222962eadd77eff4a7475ef30dbcf45618c6da1c581b01ecd7df0652ebb31ec6ca03236491e5c77c4a9de6511ee2894ae09da1a7002b36000146539f39a920f96ffb9727f94721e26b73fd66aa63125c5a4f2884ecb4c9b11b000001dd960b6c11b157d1626f0768ec099af9385aea3f31c91111a8c5b899ffb99e6b0192acd61b1853311b0bf166057ca433e231c93ab5988844a09a91c113ebc58e18019fbfd76ad6d98cafa0174391546e7022afe62e870e20e16d57c4c419a5c2bb69"
        ),
        (550000, "04c99687df30181730a1b74d57b48f97c0df1b96bb8fa7d7a23ad1720df382e5",
         "01278664cc8d581b2166cd1e1a06f87a129ca5f61575c197bf0bd979d5ac67d86101f4c1bcce00980181992cf16e481101993b258b32900426e105875bd362061c11100000017c9221fd0e10d6e46408ca079ed4d092575c01bab99760279d91a2a09de0e2260131d421582772779cebaa8260c561efa6b8141a4462b4f3944d43a250ccac993500014a41278ad3e79d44f6f92ab03dddf36ca1e02ba5b44e95f3eaf0a593d22b2c0601d8dd3c19ca05b36d4202687231d6610123d95fd6edfeccd2a61560c6dd059e58016a4238f1516a6708a8d75b06893f0201774418532b5dfc1ff1fbec670a19a54201e760ce8e5824fa9a2ae70b1ed7ecfad4c1cd2a6e9de352c29dd4013118147138012b4d55158f064e6936206f357e26afa909ba1fd7e9cdeac62eb4602603df5f6501b98b14cab05247195b3b3be3dd8639bae99a0dd10bed1282ac25b62a134afd7200000000011f8322ef806eb2430dc4a7a41c1b344bea5be946efc7b4349c1c9edb14ff9d39"
        )
    ];

    find_checkpoint(height, checkpoints)
}


fn get_main_checkpoint(height: u64) ->  Option<(u64, &'static str, &'static str)> {
    let checkpoints: Vec<(u64, &str, &str)> = vec![
        (600000, "0000001b96cc88ed39865b79c0dbdee999e1252a56513e80f74d4147939bf451",
         "01d3b69d0899d3b2a812c23def0c09aa7632cb0ec593299f4d8d6e545c36633f2f0011000001e162ba7da5a70ebaa528daf12cc93a2464385c19535ad18b79a71008746a176f01a5a8ce3bbd869afaecd611b25018ab16b53f5c7a8588846fbe26b5a66bbf7f540000012d365453fb59308f9c9665b294eb17293164c2cadad9e0c53d884e98e518b5410184b46404d973caa91670a844d689ca97f844b977dfe56c67ca1f0b4aaa2ab94200012be72e31d7db1eb1bff8c63308bbb70b8bdf597bcc8cfe9fe0e3cec0445e8d65000001e9dd3cb1e65da85f7e4dcd5479cb45a155a28795a873fa340b25a8b484ccc938019a7b8494c6dac00c1180ec6fd6765edca4f9616bcb5b1c0f8c58943dbfd93c380000011bcc61d2d87e7240c21da5f0f85fdb2d9b1806bf155da92e8f0d4de23932da08"
        ),
        (630000, "0000001efec70b964d24382dff9436138291a0d29f0b2b37b9dc8e58187394f2",
         "017e8b229c7f044b36a2f48da5c22955a9946f359818e1ee4f732e667fd0d50e3901c28397689f303da38cdccd740a542448052412e7d754b9ffe1828f7dd189b06211013f0bff67ee94046cfaad7b4562d5b4df8963b8e63445da4c2feaa3cade0f381000000000000115430c28919a755d22d52f03a63f52f89836132c48408b4500701c15cfdf895701f85eb4113a04a0c2ae3000493a09c44dbf6109ab9a72e3a70ba6b5e456a4280801626934e496c6bf071a45a722dfa3e0f7e6fe0e603d3c3e47efeeb1857e09690c0109d3c48b603a268505a5feab0db03af45ec59004ab1a221f1c92de65386a7d270001a86112ac94164cfa2f7a8bc8c70aa90c0c2f4bfad1c830ba3b30a17828b0f60e000000012bba14d7832c159b59f38f986d3ecd69cf86440efa04f8946c64cbdb5d269e70011bcc61d2d87e7240c21da5f0f85fdb2d9b1806bf155da92e8f0d4de23932da08"
        ),
    ];

    find_checkpoint(height, checkpoints)
}

fn find_checkpoint(height: u64, chkpts: Vec<(u64, &'static str, &'static str)>) -> Option<(u64, &'static str, &'static str)> {
    // Find the closest checkpoint
    let mut heights = chkpts.iter().map(|(h, _, _)| *h as u64).collect::<Vec<_>>();
    heights.sort();

    match get_first_lower_than(height, heights) {
        Some(closest_height) => {
            chkpts.iter().find(|(h, _, _)| *h ==  closest_height).map(|t| *t)
        },
        None    => None
    }
}

fn get_first_lower_than(height: u64, heights: Vec<u64>) -> Option<u64> {
    // If it's before the first checkpoint, return None. 
    if heights.len() == 0 || height < heights[0] {
        return None;
    }

    for (i, h) in heights.iter().enumerate() {
        if height < *h {
            return Some(heights[i-1]);
        }
    }

    return Some(*heights.last().unwrap());
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_lower_than() {
        assert_eq!(get_first_lower_than( 9, vec![10, 30, 40]), None);
        assert_eq!(get_first_lower_than(10, vec![10, 30, 40]).unwrap(), 10);
        assert_eq!(get_first_lower_than(11, vec![10, 30, 40]).unwrap(), 10);
        assert_eq!(get_first_lower_than(29, vec![10, 30, 40]).unwrap(), 10);
        assert_eq!(get_first_lower_than(30, vec![10, 30, 40]).unwrap(), 30);
        assert_eq!(get_first_lower_than(40, vec![10, 30, 40]).unwrap(), 40);
        assert_eq!(get_first_lower_than(41, vec![10, 30, 40]).unwrap(), 40);
        assert_eq!(get_first_lower_than(99, vec![10, 30, 40]).unwrap(), 40);
    }

    #[test]
    fn test_checkpoints() {
        assert_eq!(get_test_checkpoint(500000), None);
        assert_eq!(get_test_checkpoint(600000).unwrap().0, 600000);
        assert_eq!(get_test_checkpoint(625000).unwrap().0, 600000);
        assert_eq!(get_test_checkpoint(650000).unwrap().0, 650000);
        assert_eq!(get_test_checkpoint(655000).unwrap().0, 650000);

        assert_eq!(get_main_checkpoint(500000), None);
        assert_eq!(get_main_checkpoint(610000).unwrap().0, 610000);
        assert_eq!(get_main_checkpoint(625000).unwrap().0, 610000);
    }

}
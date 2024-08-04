# sigcrack

sigcrack is a Rust-based tool designed to find Solidity function signature collisions. This tool is useful for colliding 4 byte Keccak256 hashes, particularly in the context of Ethereum smart contracts where function signatures can be collided through bruteforce. 

## Install

To install sigcrack, simply run the following commands: 
```
git clone https://github.com/DanielBoye/sigcrack.git
cd sigcrack
./install.sh
```

## Usage

sigcrack has the following options: 
- `--target-hash <HASH>`: The target 4-byte Keccak-256 hash prefix (required).
- `--prefix <PREFIX>`: The prefix for the function names (optional).
- `--suffix <SUFFIX>`: The suffix for the function names (optional).
- `--length <LENGTH>`: The length of the guessed elements of the function name to be generated (optional, default is 6).

For detailed information on each command and its options, you can access the help message by running such commands:

```
sigcrack -h
sigcrack <COMMAND> -h
```

### Default Usage
```
sigcrack --target-hash 0x12345678
```

This command runs with only a target hash specified. It will:
- Search for function names that, when hashed with Keccak-256, produce a 4-byte prefix matching the given target hash (0x12345678).
- Use the default length of 6 characters for function names.
- Generate function names using alphanumeric characters (a-z, A-Z, 0-9).
- Continue searching until a match is found or all possibilities are exhausted.

NOTE: If no specific function parameters are provided, the tool will loop through the following default parameter types:
- `(uint256)`
- `(address)`
- `(bool)`
- `(bytes)`
- `(string)`
- `()`

## Custom length
```
sigcrack --target-hash 0x12345678 --length 8
```

This command specifies a custom length for the function names:
- Generate function names that are exactly 8 characters long.
- Use alphanumeric characters for the entire name.
- Useful when you want to constraint the exact length of the function you're looking for.

### Prefix
```
sigcrack --target-hash 0x12345678 --prefix Hello
```

- This command adds a prefix to the search:
- All generated function names will start with "Hello" now.
- The remaining characters will be filled with alphanumeric characters by the given length.
- The total length of the function name will be the prefix plus 6 characters by default.
- Useful when you want a beginning of one function name but need to find the rest to match the desired signature.

### Selected Function Parameters
```
sigcrack --target-hash 0x12345678 --suffix "(uint256)"
```

This command adds a function parameter constraint:
- All generated function names will end with the desired function parameter `(uint256)`.
- The characters before the parameters will be filled with alphanumeric characters.
- The total length of the function name will be 6 characters by default.
- Useful when you want a spesific function parameter but need a valid signature.

## With Prefix and Function Parameters
```
sigcrack --target-hash 0x12345678 --prefix Hello --suffix "(uint256)" --length 8
```
This command combines prefix, suffix, and length constraints:
- Function names will start with "Hello" and have the parameter `(uint256)`.
- The total length of the function name (excluding prefix and suffix) will be 8 characters.
- The tool will generate variations to fill the space between the prefix and suffix.
- Useful when you want the start of a function and a spesific function parameter but need to find a valid signature.

## Removal Script
This command runs a script to remove sigcrack from your system
```
./remove.sh
```
## Contributing
Contributions are welcome! Please open an issue or submit a pull request with your improvements.

## License
This project is licensed under the MIT License. See the LICENSE file for details.

## Contact
For any questions or feedback, please contact the repository owner. Happy cracking!
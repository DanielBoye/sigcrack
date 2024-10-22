# sigcrack

sigcrack is a Rust-based tool designed to find Solidity function signature collisions. This tool is useful for colliding 4 byte Keccak256 hashes, particularly in the context of Ethereum smart contracts where function signatures can be collided through bruteforce. 

## Build

You need Rust and Cargo installed on your machine. See the installation guide
[here](https://doc.rust-lang.org/cargo/getting-started/installation.html).

Then clone the repo and install the CLI globally like this:
```sh
cargo install --path .
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
- Search for function names that, when hashed with Keccak-256, produce a 4-byte prefix matching the given target hash `0x12345678`.
- Use the default length of 6 characters for function names.
- Generate function names using alphanumeric characters (a-z, A-Z, 0-9).
- Continue searching until a match is found sooner or later.

### Default Parameter Types

If no specific function parameters are provided, the tool will loop through the following default parameter types: `uint256`, `address`, `bool`, `bytes`, `string`, and `()`.

## Time Complexity and Expected Time
The brute-force algorithm used in sigcrack has an overall time complexity similar to a normal brute-force attack, which can be expressed as:
$$O(m \cdot n)$$
However, for sigcrack, the time complexity is more specifically:
$$O(62^{(L - P - S)} \cdot C)$$
Where:
- $L$ is the total length of the function name,
- $P$ is the length of the prefix (if any),
- $S$ is the length of the suffix (if any),
- $C$ is the constant time taken to compute the keccak256 hash, which is $O(1)$.

The term $62^{(L - P - S)}$ represents the number of possible combinations of alphanumeric characters (26 lowercase + 26 uppercase + 10 digits = 62 characters) for the remaining length of the function name after accounting for the prefix and suffix.

The term $C$ represents the constant time required to compute the hash for each generated function name.

In summary, the time complexity indicates that the number of combinations grows exponentially with the length of the function name minus the lengths of the prefix and suffix, and each combination requires a constant amount of time to compute its hash.

## hash go brrr

Given a speed of approximately **4.4 MH/s**, the expected time to find a matching hash can be estimated based on the number of combinations generated. For example, if you are searching for function names of length 6 with no prefix or suffix, the total combinations would be $62^6$, which is $56,800,235,584$. 
To calculate the expected time:
1. Total combinations: $$62^6 \approx 56.8 \text{ billion}$$
2. Speed: $$4.4 \text{ MH/s}$$
3. Expected time (in seconds): 
   $$ \text{Expected Time} = \frac{56,800,235,584}{4,400,000} \approx 12,909 \text{ seconds} \approx 3.6 \text{ hours} $$

For a 50% chance of finding a match it will take:
$$ \text{Expected Time}_{50\%} = \frac{3.6 \text{ hours}}{2} \approx 1.8 \text{ hours} $$


This means that on average it will take approximately 3 hours to guarantee finding a matching function signature. The actual time can vary depending on the length and constraints provided. Some searches may take around 10 minutes, while others may take up to 5 hours. In my experience, it usually takes around 45 minutes.

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
- Useful when you want the start of a function and a specific function parameter but need to find a valid signature.

## Uninstall
To uninstall the program, simply run:
```sh
cargo uninstall sigcrack
```

## Contributing
Contributions are welcome! Open an issue or submit a pull request with your improvements. 

## License
This project is licensed under the MIT License. See the LICENSE file for details.

## Contact
For any questions or feedback, please take contact. Happy cracking!
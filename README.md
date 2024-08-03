## sigcrack

sigcrack is a Rust-based tool designed to find Solidity function signature collisions. 

### installation 
#### clone the repository

```bash
git clone https://github.com/DanielBoye/sigcrack.git
cd sigcrack
```

#### run the installation script
```bash
./install.sh
```

### Usage
Once installed, you can run sigcrack from the command line. Here are some examples:


Basic Usage

```bash
sigcrack --target-hash 0x64d98f6e
```

With Prefix
```bash
sigcrack --target-hash 0x64d98f6e --prefix Hello
```

With Suffix
```bash
sigcrack --target-hash 0x64d98f6e --suffix "(uint256)"
```

With Custom Length
```bash
sigcrack --target-hash 0x64d98f6e --length 8
```

With Prefix and Suffix
```bash
sigcrack --target-hash 0x64d98f6e --prefix Hello --suffix "(uint256)" --length 8
```

To remove sigcrack from your system, use the provided removal script:
```bash
./remove.sh
```
Contributing
Contributions are welcome! Feel free to open issues or submit pull requests on GitHub.
License
This project is licensed under the MIT License.
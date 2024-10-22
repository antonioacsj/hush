# Rash - Rush Hash

Rash is a high-performance hashing algorithm designed to handle increasingly larger files by utilizing parallel computation. It divides files into blocks, hashes them in parallel, and then combines the block hashes into a final hash, similar to how a blockchain works.

## Supported Algorithms

- **rsha256** (default) - Hash the blocks with sha256
- **rsha512** (planned support) - Hash the blocks with sha512

## How It Works

Rash uses multi-threaded processing to hash file blocks, improving speed on large files. After hashing each block, it generates a final hash by combining the hashes of all blocks in sequence.

### Parameters

- `blockSize`: Defines the size of the chunks the file will be split into for hashing.

### Example Usage

```bash
# Using rsha256
rash rsha256 <path_to_file>

# Using rsha256 with a block size of 64MB
rash rsha256 --blockSize 64MB <path_to_file>

# Using rsha256 to check a file with hash generated:
rash check <path_to_file_with_hash>

```

#Obs: Block size in verification has to be the same of generation. So it will use of line with format

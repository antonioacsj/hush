# rash - Hash tool for rush people

`rash` is a hashing algorithm designed to handle increasingly larger files by utilizing parallel/concurrent computation and make the work faster.

To files smaller than block size, computes hashes in a single block, in a multithread environment.
To big files, it divides into blocks, hashes them in a multithread/multiworker, and then combines the block hashes into a final hash, similar to how a blockchain works. (See picture.)

## How It Works

Rash uses multi-threaded processing to hash file blocks, improving speed on large files. After hashing each block, it generates a final hash by combining the hashes of all blocks in sequence.

![alt text](https://github.com/antonioacsj/rash/blob/master/etc/Blocks.jpg?raw=true)

## Supported Algorithms

- **rsha256** (default) - Hash the blocks with sha256
- **rsha512** (planned support) - Hash the blocks with sha512

### Parameters to play :)

- `blocksize`: Defines the size of the chunks the file will be split into for hashing.
- `buffersize`: Defines the size of the buffer the file will be read into.
- `n_workers`: Defines the amount of workers to use in main pool.
- `n_max_concur`: Defines the amount of maximum concurrent access to each file (threads to each file).

### Example Usage

```bash
# Using rsha256
rash --help

# Using log, n_workers=2 and n_max_concur=5
rash gen <path_to_hash> --log --n_workers 2 --n_max_concur 5

# Using rsha256 with a block size of 64MB
rash rsha256 --blockSize 64MB <path_to_file>

# Using rsha256 to check a file with hash generated:
rash check <path_to_file_with_hash>

```

#Obs: The block size in verification has to be the same of generation. This information is showed in result and, by default, its 50MB. When save the hash, save the blockSize information too.

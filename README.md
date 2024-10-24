# hush - Hash tool for rush situations

`hush` is a hashing algorithm designed to be faster than usual.

### ⚠️ If you know a fast way to hash big amount of files/data in ordinary computers, please, let me know!

### How It Works

To files smaller than block size, computes hashes in a single block.

To big files, it divides into blocks, hashes each block, and then combines this hashes into a final hash, similar to how a blockchain works. (See picture.)

### Blocksize

The blocksize is whats differs hush of traditional hash algorithms.
Valid values: 15KB 10MB, 1GB, etc.
where: **B = 1 Byte**

All work is made in a multithread environment, to maximize speed.

![alt text](https://github.com/antonioacsj/rash/blob/master/etc/Blocks.jpg?raw=true)

### ⚠️ Important Note: When save the hash, save the information of algorithm and blocksize used !

The block size in verification process **NEEDS to be the same of generation** (of course!). So, this information is showed in the result. Register the hash with algorithm and block!

## Supported Algorithms

- **rsha256-Blocksize** (default) - Hash file in blocks with sha256 of size: Bloksize
- **others** (planned support) - Hash the blocks with others hash algorithms

### Example Usage

```bash
# Using rsha256
hush --help

# Using log, n_workers=2 and n_max_concur=5
hush gen <path_to_hash> --log --n_workers 2 --n_max_concur 5

# Using rsha256 with a block size of 64MB
hush rsha256 <path_to_file> --blocksize 64MB

# Using rsha256 to check a file with hash generated:
hush check <file_with_hash> <work_dir_base> <parameters>

```

### Parameters to play :)

- `blocksize`: Defines the size of the chunks the file will be split into for hashing.
- `buffersize`: Defines the size of the buffer the file will be read into.
- `n_workers`: Defines the amount of workers to use in main pool.
- `n_max_concur`: Defines the amount of maximum concurrent access to each file.

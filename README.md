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

### ⚠️ Register the hash with algorithm and block!

The block size in verification process **NEEDS to be the same of generation** (of course!). So, this information is showed in the result.

## Supported Algorithms to hash Blocks

- **sha256** (default) -

### Example Usage

```bash
# Help
hush --help

# Generating hashes
hush gen <path_to_hash>

# Generating hashes saving result in a file named hashes.txt
hush gen <path_to_hash>  > hashes.txt

#Checking hashes in 'hashes.txt' using a dir as base of work
hush check hashes.txt <work_dir_base>

# Generating hashes with options
hush gen <path_to_hash> --log --n_workers 2 --n_max_concur 5

```

### Parameters to play :)

- `blocksize`: Defines the size of the chunks the file will be split into for hashing.
- `buffersize`: Defines the size of the buffer the file will be read into.
- `n_workers`: Defines the amount of workers to use in main pool.
- `n_max_concur`: Defines the amount of maximum concurrent access to each file.

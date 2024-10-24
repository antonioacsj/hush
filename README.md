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

### ⚠️ Register the hash with algorithm and blocksize! ⚠️

The blocksize in verification process **NEEDS to be the same of generation** (of course!).

This information is showed in the result of generation process. **SAVE ALL DATA**.

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

#Simple use
hush gen C:/Folder1/Data > C:/Folder1/hash_file.txt
hush check C:/Folder1/hash_file.txt C:/Folder1/Data

#Use with options:
hush gen C:/Folder1/Data > C:/Folder1/hash_file.txt --progress --stop --log --blocksize 100MB --n_workers 40 --n_max_concur 20
hush check C:/Folder1/hash_file.txt C:/Folder1/Data --progress --stop --log --n_workers 40 --n_max_concur 20

```

### Options to play :)

'--log' to print lots of boring stuff
'--progress' to show that something is being done while you drink coffee.
'--stop' Stop everything if some error. By default, don´t stop. (Make it in your way!)
'--blocksize Value' to change size that file block is divided. Default 50MB. Use KB, MB, GB, TB, where B is Byte, ok? :)
'--buffersize Value' to change buffersize to read buffers.. Default 10KB. Use KB, MB, GB, TB. Hands off if you don't know what it is.
'--n_workers Value' to change how many workers will be used in main pool. Default 15.
'--n_max_concur Value' to change how maximum number of concurrent access to each file, in pool of slaves. Default 15
'--hash_alg Value' to change hash function to use. By default and supported: sha256

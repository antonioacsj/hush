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

This information is showed in the result of generation process. **SAVE ALL DATA**, like below:

```bash
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 ?sha256*./sub1/sub2/Novo(a) Imagem de bitmap.bmp
0a43092d0fc7036e79e45654d5a78b05caff3da5963542a7de3735d5cea7bda9 ?sha256*./sub1/hashs_saida.txt
ccf0518aaae1687f2a17cb5f5d63fce1c80f03d99ab928d899144e7cf0cd852f ?sha256*./sub1/sub2/comp_rsha256.txt
143d834b9818464dc13b6eff0ae55a6761f3ec97dcd37b5408457db4906aa02f ?sha256*./sub1/sub2/comp_hash_saida_fsum.txt
5630d9f4b413c1986a42d4b6b69c758f8bd32ebf6ff7316d133a6cde0951cf94 ?hsha256-50MB*./65MB
281d5d93464f1165ea7c403ca99d63ff4bf9a360864f8df4bd0e8e6c03774e98 ?hsha256-50MB*./196MB

```

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

```bash
'--log' to print lots of boring stuff
'--progress' to show that something is being done while you drink coffee.
'--stop' Stop everything if some error. By default, don´t stop. (Make it in your way!)
'--blocksize Value' to change size that file block is divided. Default 50MB. Use KB, MB, GB, TB, where B is Byte, ok? :)
'--buffersize Value' to change buffersize to read buffers.. Default 10KB. Use KB, MB, GB, TB. Hands off if you don't know what it is.
'--n_workers Value' to change how many workers will be used in main pool. Default 15.
'--n_max_concur Value' to change how maximum number of concurrent access to each file, in pool of slaves. Default 15
'--hash_alg Value' to change hash function to use. By default and supported: sha256

```

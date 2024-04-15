# lfs-dal

A custom transfer agent for Git LFS powered by OpenDAL.

## Overview

* Git LFS can switch the storage backend by [custom transfer agents][custom-transfer].
* OpenDAL provides access to [many storage systems][services] such as Azure, GCP, AWS S3, WebDAV, Dropbox, Google Drive, One Drive, etc.
* lfs-dal enables you to store LFS data on many storage systems!

## Installation

Download the released binary or build from source.

```bash
# build from source
$ cargo install lfs-dal
```

## Usage

### `git-lfs` configuration

Configure `git-lfs` to use `lfs-dal` as a custom transfer agent.

```bash
$ git lfs install --local
$ git config lfs.standalonetransferagent lfs-dal
$ git config lfs.customtransfer.lfs-dal.path /PATH/TO/LFS-DAL
$ git config -f .lfsconfig lfs.url lfs-dal  # avoid to push to the default server accidentally
```

### `lfs-dal` configuration

Configure OpenDAL service settings in `lfs-dal` section at `.lfsdalconfig` or `.git/config`.
`lfs-dal --list` shows available schemes.
For service-specific options, refer to the [OpenDAL documentation][services].

> \[!NOTE]
> gitconfig does not allow `_` in keys. Use `-` instead.

> \[!NOTE]
> lfs-dal looks for `.lfsdalconfig` and `.git/config`.
> `.git/config` is not committed to the repository.

> \[!CAUTION]
> Do not commit your credentials to the repository.
> Some OpenDAL services support importing credentials via environment variables.

### AWS S3 example

```bash
$ git config -f .lfsdalconfig lfs-dal.scheme s3
$ git config -f .lfsdalconfig lfs-dal.bucket test
$ git config -f .lfsdalconfig lfs-dal.region us-east-1
$ git config lfs-dal.access-key-id 123456
$ git config lfs-dal.secret-access-key 123456
```

### Using AWS SSO credentials

Use [aws2-wrap](https://github.com/linaro-its/aws2-wrap).

```bash
$ aws2-wrap git lfs pull
```

## Debugging

For testing OpenDAL settings, run `lfs-dal --exit`. It exits immediately after initializing OpenDAL.

For more detailed log, configure `lfs-dal` to output log to a file.

```bash
$ git config lfs.customtransfer.lfs-dal.args "--log-output=log-lfs-dal.txt --log-level=debug"
$ git config lfs.customtransfer.lfs-dal.concurrent false  # avoid log interleaving
```

## Alternatives

* [rudolfs][rudolfs] is not a LFS transfer agent but a LFS server implemented in Rust. It supports AWS S3 and local file system.
* [lfsrclone][lfsrclone] is a LFS transfer agent implemented in Python. It runs `rclone` command to transfer data.
* [lfs-os][lfs-os] is a LFS transfer agent implemented in Rust. It uses `object_store` crate.

[custom-transfer]: https://github.com/git-lfs/git-lfs/blob/main/docs/custom-transfers.md

[services]: https://opendal.apache.org/docs/category/services/

[rudolfs]: https://github.com/jasonwhite/rudolfs

[lfsrclone]: https://github.com/Jwink3101/lfsrclone

[lfs-os]: https://github.com/hacksadecimal/lfs-os

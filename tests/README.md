# Testing with real-world data

The [`libsml-testing`](https://github.com/devZer0/libsml-testing) repository contains data dumps of many real-world smart meters. This data is included as a `git subtree` here and used for integration testing.

To update the subtree, run the following git command:

```bash
git subtree pull --prefix tests/libsml-testing https://github.com/devZer0/libsml-testing.git master --squash
```
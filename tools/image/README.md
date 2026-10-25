# Building and releasing the image

Build and push from your own machine. It is faster than a GitHub runner (the build cache stays on
your disk) and costs no Actions minutes.

## One-time: log in to ghcr.io

Create a personal access token (classic) with the scopes `write:packages` and `read:packages`
(GitHub -> Settings -> Developer settings -> Personal access tokens). Then:

```bash
echo "$TOKEN" | docker login ghcr.io -u <your github user> --password-stdin
```

## Release

```bash
tools/image/build.sh --tag ghcr.io/<owner>/<repo>:1.0.0-rc1 --push
```

- Image names must be lowercase.
- A tag that you pushed once is never overwritten. Roll out with a new tag, move servers one at a
  time (staging first), and go back by pointing a server at the previous tag.
- Pin a server to the digest (`image: ghcr.io/<owner>/<repo>@sha256:...`) for an exact, repeatable
  rollout. `docker buildx imagetools inspect <tag>` shows it.
- `--fast` builds a quicker test image (thin LTO). Use it to try something, never to roll out.

## Speed

- The first build compiles every dependency (about half an hour). The next ones reuse them from the
  BuildKit cache and only rebuild what changed, plus the final link.
- The release profile uses fat LTO and one codegen unit, which is what makes the last step slow.
  `--fast` trades a bit of runtime speed for a much shorter build.
- Do not run `docker builder prune` / `docker system prune -a` between builds, it deletes the cache.

## Packages in ghcr

A package is private after the first push. To keep it next to the repository, the image has the
`org.opencontainers.image.source` label; set `SOURCE` (a build arg) to the URL of your repository.

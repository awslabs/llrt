<!--  markdownlint-disable MD043 -->

## Table of contents <!-- omit in toc -->

- [Overview](#overview)
- [Current Maintainers](#current-maintainers)
- [Labels](#labels)
- [Maintainer Responsibilities](#maintainer-responsibilities)
  - [Uphold Code of Conduct](#uphold-code-of-conduct)
  - [Prioritize Security](#prioritize-security)
  - [Review Pull Requests](#review-pull-requests)
  - [Triage New Issues](#triage-new-issues)

## Overview

> **Please treat this content as a living document.**

This is document explains who the maintainers are (see below), what they do in this repo, and how they should be doing it. If you're interested in contributing, see [CONTRIBUTING](CONTRIBUTING.md).

## Current Maintainers

| Maintainer      | GitHub ID                                           | Affiliation |
| --------------- | --------------------------------------------------- | ----------- |
| Richard Davison | [richarddavison](https://github.com/richarddavison) | Amazon      |
| Nik Pinski      | [nikp](https://github.com/nikp)                     | Amazon      |

## Labels

> WORK IN PROGRESS

## Maintainer Responsibilities

Maintainers are active and visible members of the community, and have [maintain-level permissions on a repository](https://docs.github.com/en/organizations/managing-access-to-your-organizations-repositories/repository-permission-levels-for-an-organization). Use those privileges to serve the community and evolve code as follows.

Be aware of recurring ambiguous situations and [document them](#common-scenarios) to help your fellow maintainers.

### Uphold Code of Conduct

Model the behavior set forward by the [Code of Conduct](CODE_OF_CONDUCT.md) and raise any violations to other maintainers and admins. There could be unusual circumstances where inappropriate behavior does not immediately fall within the [Code of Conduct](CODE_OF_CONDUCT.md).

These might be nuanced and should be handled with extra care - when in doubt, do not engage and reach out to other maintainers and admins.

### Prioritize Security

Security is your number one priority. Maintainer's Github keys must be password protected securely and any reported security vulnerabilities are addressed before features or bugs.

Note that this repository is monitored and supported 24/7 by Amazon Security, see [Reporting a Vulnerability](CONTRIBUTING.md#security-issue-notifications) for details.

### Review Pull Requests

Review pull requests regularly, comment, suggest, reject, merge and close. Accept only high quality pull-requests. Provide code reviews and guidance on incoming pull requests.

PRs are [labeled](#labels) based on file changes and semantic title. Pay attention to whether labels reflect the current state of the PR and correct accordingly.

Use and enforce [semantic versioning](https://semver.org/) pull request titles, as these will be used for [CHANGELOG](CHANGELOG.md) and [Release notes](https://github.com/awslabs/llrt/releases) - make sure they communicate their intent at the human level.

See [Common scenarios](#common-scenarios) section for additional guidance.

### Triage New Issues

Manage [labels](#labels), review issues regularly, and create new labels as needed by the project. Remove `triage` label when you're able to confirm the validity of a request, a bug can be reproduced, etc. Give priority to the original author for implementation, unless it is a sensitive task that is best handled by maintainers.

Make sure issues are assigned to our [board of activities](https://github.com/orgs/awslabs/projects/145/)

Use our [labels](#labels) to signal good first issues to new community members, and to set expectation that this might need additional feedback from the author, other customers, experienced community members and/or maintainers.

> WORK IN PROGRESS

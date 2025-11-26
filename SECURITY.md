# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in sketch_oxide, please report it responsibly and do not disclose it publicly until a fix has been released.

### How to Report

1. **Do NOT open a public GitHub issue** for security vulnerabilities
2. **Email**: Send your report to [security@sketch-oxide.dev]
3. **Include**:
   - Description of the vulnerability
   - Steps to reproduce (if applicable)
   - Affected versions
   - Potential impact
   - Suggested fix (if you have one)

### Response Timeline

- **Initial response**: Within 24 hours
- **Assessment**: Within 48 hours
- **Fix**: Target 30 days for patch release
- **Disclosure**: We will coordinate with you on timing

## Supported Versions

| Version | Status | Security Updates |
|---------|--------|------------------|
| 0.1.x | Current | Yes |
| 0.0.x | End of Life | No |

We recommend always using the latest version to ensure you have all security fixes.

## Security Best Practices

When using sketch_oxide:

1. **Keep Dependencies Updated**: Regularly update sketch_oxide and its dependencies
2. **Validate Input**: Always validate and sanitize input data
3. **Use TLS for Data Transmission**: When transmitting sketched data over networks, use TLS/SSL
4. **Review Performance Guarantees**: Be aware that sketches trade accuracy for memory/speed
5. **Monitor False Positive Rates**: For membership filters, monitor actual false positive rates against expected values

## Known Limitations

- **Probabilistic Data Structures**: By design, these are approximate algorithms. They have:
  - Expected false positive rates
  - Potential for off-by-one errors in cardinality estimates
  - Tunable accuracy/space trade-offs

- **Adversarial Attacks**: Some sketches (e.g., Bloom filters) can be optimized against with adversarial input
  - Use multiple independent hash functions
  - Consider randomized salts

- **Serialization**: Be cautious when deserializing sketches from untrusted sources
  - Validate magic bytes/versions
  - Use strict bounds checking

## Dependency Security

sketch_oxide has minimal dependencies:
- **Rust core**: No external dependencies (except standard library)
- **Python bindings**: PyO3 (security maintained by PyO3 team)
- **Node.js bindings**: napi-rs (security maintained by napi-rs team)
- **Java bindings**: JNI (standard Java library)
- **C# bindings**: .NET Framework (security maintained by Microsoft)

We regularly audit dependencies for known vulnerabilities.

## Disclosure Timeline

When a security vulnerability is confirmed:

1. **Day 0**: Vulnerability confirmed, assessment begins
2. **Days 1-3**: Fix is developed and tested
3. **Days 4-5**: Security advisory is prepared
4. **Day 30**: Public disclosure and patch release (coordinated with reporters)

## Security Advisories

Past security advisories will be published at:
https://github.com/sketch-oxide/sketch-oxide/security/advisories

Subscribe to repository notifications to be alerted of new advisories.

## PGP Key

For PGP-signed communications regarding security:
```
[PGP key information to be added upon setup]
```

## Third-Party Audits

As sketch_oxide matures, we plan to conduct regular third-party security audits. Results will be published at https://github.com/sketch-oxide/sketch-oxide/security.

## Changelog Security Information

See [CHANGELOG.md](CHANGELOG.md) for security-related changes in each release, marked with **[SECURITY]** tags.

## Questions?

For general security questions that are not vulnerability reports, please file a discussion on GitHub at https://github.com/sketch-oxide/sketch-oxide/discussions.

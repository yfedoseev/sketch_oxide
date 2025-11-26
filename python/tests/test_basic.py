"""Basic tests for sketch_oxide Python module."""

import pytest


def test_module_imports() -> None:
    """Test that the sketch_oxide module can be imported."""
    try:
        import sketch_oxide

        # Check if this is the built extension module (not just the stub package)
        if not hasattr(sketch_oxide, "__version__"):
            pytest.skip("Module not built yet - run 'maturin develop' first")

        assert hasattr(sketch_oxide, "__version__")
        assert hasattr(sketch_oxide, "__doc__")
    except ImportError:
        pytest.skip("Module not built yet - run 'maturin develop' first")


def test_version_exists() -> None:
    """Test that version string is present."""
    try:
        import sketch_oxide

        # Check if this is the built extension module
        if not hasattr(sketch_oxide, "__version__"):
            pytest.skip("Module not built yet - run 'maturin develop' first")

        version = sketch_oxide.__version__
        assert isinstance(version, str)
        assert len(version) > 0
    except ImportError:
        pytest.skip("Module not built yet")

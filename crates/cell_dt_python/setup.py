from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="cell_dt",
    version="0.1.0",
    description="Cell Differentiation Platform - Python bindings",
    rust_extensions=[
        RustExtension(
            "cell_dt",
            binding=Binding.PyO3,
            path="Cargo.toml",
        )
    ],
    packages=[],
    zip_safe=False,
    python_requires=">=3.7",
    install_requires=[],
    extras_require={
        "analysis": ["pandas", "numpy", "matplotlib", "scipy"],
        "jupyter": ["jupyter", "ipywidgets", "plotly"],
    },
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Science/Research",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Rust",
        "Topic :: Scientific/Engineering :: Bio-Informatics",
    ],
)

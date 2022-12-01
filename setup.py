from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    rust_extensions=[
        RustExtension(
            target="elk_mq",
            features=[ "python_bindings" ],
            binding=Binding.RustCPython
        )
    ],
    zip_safe=False
)

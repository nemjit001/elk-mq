use cpython::{ py_module_initializer };

py_module_initializer!(
    elkmq,
    | py, module | {
        module.add(py, "__doc__", "ElkMQ python module")?;

        Ok(())
    }
);

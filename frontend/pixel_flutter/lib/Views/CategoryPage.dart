import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart' show BlocBuilder;
import 'package:pixel_flutter/blocs/CategoryBlocs.dart';

class CategoryPage extends StatefulWidget {
  @override
  _CategoryPageState createState() => _CategoryPageState();
}

class _CategoryPageState extends State<CategoryPage> {

  final CategoryBloc _categoryBloc = CategoryBloc();

  _CategoryPageState() {
    _categoryBloc.dispatch(GetCategories());
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _categoryBloc,
        builder: (BuildContext context, CategoryState state) {
          return Scaffold(
              body:  Center(
                child: CircularProgressIndicator(),
              )
          );
        });
  }

  @override
  void dispose() {
    _categoryBloc.dispose();
    super.dispose();
  }
}

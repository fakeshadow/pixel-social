import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart' show BlocBuilder;

import 'package:pixel_flutter/blocs/CategoryBlocs.dart';
import 'package:pixel_flutter/components/NavigationBar/NavBarCommon.dart';
import 'package:pixel_flutter/components/NavigationBar/TabNavBar.dart';

class CategoryPage extends StatefulWidget {
  @override
  _CategoryPageState createState() => _CategoryPageState();
}

class _CategoryPageState extends State<CategoryPage> {
  final _scaffoldKey = new GlobalKey<ScaffoldState>();
  final CategoryBloc _categoryBloc = CategoryBloc();

  final _scrollController = ScrollController();

  _CategoryPageState() {
    _categoryBloc.dispatch(LoadCategories());
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _categoryBloc,
        builder: (BuildContext context, CategoryState state) {
          return Scaffold(
            key: _scaffoldKey,
            bottomNavigationBar: TabNavBar(1),
            endDrawer: Container(
              child: Center(child: Text('abcdefg')),
            ),
            body: CustomScrollView(
                controller: _scrollController,
                slivers: <Widget>[
                  NavBarCommon(title: 'test', isClose: false),
                  CategoryList(state)
                ]),
          );
        });
  }

  @override
  void dispose() {
    _categoryBloc.dispose();
    super.dispose();
  }
}

class CategoryList extends StatelessWidget {
  final state;

  CategoryList(this.state);

  @override
  Widget build(BuildContext context) {
    return null;
  }
}

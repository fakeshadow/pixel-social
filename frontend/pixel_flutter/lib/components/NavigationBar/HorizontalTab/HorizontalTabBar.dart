import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/blocs/CategoryBlocs.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter/blocs/HorizontalTabBlocs.dart';
import 'package:pixel_flutter/components/Categories/CategoryCard.dart';
import 'package:pixel_flutter/components/NavigationBar/HorizontalTab/HorizontalTabText.dart';

class HorizontalTab extends StatefulWidget {
  @override
  _HorizontalTabState createState() => _HorizontalTabState();
}

class _HorizontalTabState extends State<HorizontalTab> {
  CategoryBloc _categoryBloc;
  ErrorBloc _errorBloc;

  @override
  void initState() {
    _categoryBloc = CategoryBloc();
    _errorBloc = ErrorBloc();
    _categoryBloc.dispatch(LoadCategories());
    super.initState();
  }

  @override
  void dispose() {
    _categoryBloc.dispose();
    _errorBloc.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return BlocProvider(
      bloc: HorizontalTabBloc(),
      child: Container(
        height: 450,
        child: BlocBuilder(
            bloc: _categoryBloc,
            builder: (BuildContext context, CategoryState state) {
              if (state is CategoryFailed) {
                _errorBloc.dispatch(GetError(error: state.error));
                //ToDo: change to navigator pop or push user back to homepage later
                return Container();
              }
              if (state is CategoryLoading) {
                return Container(
                    child: Center(child: CircularProgressIndicator()));
              }
              if (state is CategoryLoaded) {
                return Stack(
                  children: <Widget>[
                    Positioned(
                      left: 0.05,
                      top: 10,
                      bottom: 0,
                      width: 70,
                      child: Padding(
                        padding: EdgeInsets.symmetric(vertical: 70.0),
                        child: Column(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: <Widget>[
                            HorizontalTabText(
                              text: 'Media',
                              index: 0,
                            ),
                            HorizontalTabText(
                              text: 'Forum',
                              index: 1,
                            ),
                            HorizontalTabText(
                              text: 'Info',
                              index: 2,
                            ),
                          ],
                        ),
                      ),
                    ),
                    Padding(
                      padding: EdgeInsets.only(left: 60),
                      child: ListView(
                        scrollDirection: Axis.horizontal,
                        children: <Widget>[
                          CategoryCard(),
                          CategoryCard(),
                        ],
                      ),
                    )
                  ],
                );
              }
            }),
      ),
    );
  }
}

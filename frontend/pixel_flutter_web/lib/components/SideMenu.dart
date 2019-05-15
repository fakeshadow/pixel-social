import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/CategoryBlocs.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';

import 'package:pixel_flutter_web/env.dart';
import 'package:pixel_flutter_web/models/Category.dart';
import 'package:pixel_flutter_web/views/TopicsPage.dart';

class SideMenu extends StatelessWidget with env {
  @override
  Widget build(BuildContext context) {
    return Padding(
        padding: const EdgeInsets.only(left: 30.0),
        child: Container(
          width: 200,
          child: ListView(children: <Widget>[
            Divider(),
            SideMenuBox(
                sideMenuItem: SideMenuItem(
                    title: 'Popular Categories',
                    blocBuilder: CategoryBuilder(context),
                    // ToDo: Hero widget and push to categories page.
                    onTapBottom: () => print("testtest"))),
            Divider(),
            BlocBuilder(
              bloc: BlocProvider.of<UserBloc>(context),
              builder: (context, state) {
                if (state is UserLoaded) {
                  return SideMenuBox(
                      sideMenuItem: SideMenuItem(
                          title: 'My collection',
                          blocBuilder: CollectionBuilder(context),
                          // ToDo: Hero widget and push to categories page.
                          onTapBottom: () => print("testtest")));
                } else {
                  return Container();
                }
              },
            )
          ]),
        ));
  }

  Widget Divider() {
    return SizedBox(
      height: 20,
    );
  }

  Widget CategoryBuilder(context) {
    return BlocBuilder(
      bloc: BlocProvider.of<CategoryBloc>(context),
      builder: (context, state) {
        if (state is CategoryFailed) {
          BlocProvider.of<ErrorBloc>(context)
              .dispatch(GetError(error: "Failed to load categories"));
          return Container();
        } else if (state is CategoryInit || state is CategoryLoading) {
          return Container(child: CircularProgressIndicator());
        } else if (state is CategoryLoaded) {
          return ListView.builder(
            physics: ClampingScrollPhysics(),
            shrinkWrap: true,
            itemCount: state.categories.length > 10 ? 10 : state.categories.length,
            itemBuilder: (context, index) {
              return InkWell(
                onTap: () => pushToTopicsPage(context, state.categories[index]),
                child: Stack(
                  children: <Widget>[
                    Image.network(
                        url + 'public/${state.categories[index].thumbnail}',
                        width: 200,
                        height: 40,
                        fit: BoxFit.fitWidth),
                    Container(
                      width: 200,
                      height: 40,
                      color: Colors.black26.withOpacity(0.4),
                    ),
                    Positioned(
                      top: 10,
                      left: 30,
                      child: Text(
                        state.categories[index].name,
                        style: TextStyle(color: Colors.white, fontSize: 16),
                      ),
                    ),
                  ],
                ),
              );
            },
          );
        } else {
          return Container();
        }
      },
    );
  }

  // ToDo: Add user collection in backend
  Widget CollectionBuilder(context) {
    return ListView.builder(
      physics: ClampingScrollPhysics(),
      shrinkWrap: true,
      itemCount: 5,
      itemBuilder: (context, index) {
        return InkWell(
          onTap: () => {},
          child: Stack(
            children: <Widget>[
              // Todo: give an drag drop notification if user have no collection
                    Image.network(
                        url,
                        width: 200,
                        height: 40,
                        fit: BoxFit.fitWidth),
                    Container(
                      width: 200,
                      height: 40,
                      color: Colors.black26.withOpacity(0.4),
                    ),
              Positioned(
                top: 10,
                left: 30,
                child: Text(
                  "test",
                  style: TextStyle(color: Colors.white, fontSize: 16),
                ),
              ),
            ],
          ),
        );
      },
    );
  }

  void pushToTopicsPage(BuildContext context, Category category) {
    Navigator.push(
        context,
        MaterialPageRoute(
            builder: (context) => TopicsPage(category: category)));
  }
}

class SideMenuBox extends StatelessWidget {
  final Widget sideMenuItem;

  SideMenuBox({this.sideMenuItem});

  @override
  Widget build(BuildContext context) {
    return Container(
        decoration: BoxDecoration(
            border: Border.all(
              color: Colors.black,
              width: 1.0,
            ),
            boxShadow: [
              new BoxShadow(
                color: Colors.black12,
                blurRadius: 2.0,
              ),
            ],
            shape: BoxShape.rectangle),
        child: sideMenuItem);
  }
}

class SideMenuItem extends StatelessWidget {
  final String title;
  final Widget blocBuilder;
  final Function onTapBottom;

  SideMenuItem({this.title, this.blocBuilder, this.onTapBottom});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Container(
          padding: const EdgeInsets.only(left: 10.0),
          alignment: Alignment.centerLeft,
          height: 20,
          child: Text(title),
        ),
        blocBuilder,
        Container(
            padding: const EdgeInsets.only(right: 10),
            alignment: Alignment.centerRight,
            height: 25,
            child: InkWell(
              // ToDo: Hero widget and push to categories page.
              onTap: onTapBottom,
              child: Text('...Load more', style: TextStyle(fontSize: 15)),
            )),
      ],
    );
  }
}
